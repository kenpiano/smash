//! Request/response dispatcher for LSP.
//!
//! Tracks pending requests by ID, routes responses to waiting callers
//! via oneshot channels, and handles server-initiated notifications.
use std::collections::HashMap;

use tokio::sync::oneshot;

use crate::error::LspError;
use crate::transport::{JsonRpcMessage, RpcError};

/// Callback type for handling notifications from the server.
pub type NotificationHandler = Box<dyn Fn(String, serde_json::Value) + Send + Sync>;

/// Manages pending requests and routes responses.
pub struct Dispatcher {
    /// Map of request ID to pending response sender.
    pending: HashMap<i64, oneshot::Sender<DispatchResult>>,
    /// Handler for server-initiated notifications.
    notification_handler: Option<NotificationHandler>,
}

/// The result dispatched to a waiting request.
#[derive(Debug)]
pub enum DispatchResult {
    /// Successful response with the result value.
    Success(serde_json::Value),
    /// Error response from the server.
    Error(RpcError),
}

impl Dispatcher {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            notification_handler: None,
        }
    }

    /// Set the handler for server-initiated notifications.
    pub fn set_notification_handler(&mut self, handler: NotificationHandler) {
        self.notification_handler = Some(handler);
    }

    /// Register a pending request and return a receiver for the response.
    pub fn register_request(&mut self, id: i64) -> oneshot::Receiver<DispatchResult> {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);
        rx
    }

    /// How many requests are pending.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Route an incoming message to the appropriate handler.
    ///
    /// - Responses are matched to pending requests by ID.
    /// - Notifications are forwarded to the notification handler.
    /// - Requests from the server are logged but not handled.
    pub fn dispatch(&mut self, message: JsonRpcMessage) -> Result<(), LspError> {
        match message {
            JsonRpcMessage::Response { id, result, error } => {
                if let Some(sender) = self.pending.remove(&id) {
                    let dispatch_result = if let Some(err) = error {
                        DispatchResult::Error(err)
                    } else {
                        DispatchResult::Success(result.unwrap_or(serde_json::Value::Null))
                    };
                    // If the receiver was dropped, that's ok
                    let _ = sender.send(dispatch_result);
                    Ok(())
                } else {
                    tracing::warn!("received response for unknown request id: {}", id);
                    Ok(())
                }
            }
            JsonRpcMessage::Notification { method, params } => {
                if let Some(handler) = &self.notification_handler {
                    handler(method, params);
                } else {
                    tracing::debug!("unhandled notification: {}", method);
                }
                Ok(())
            }
            JsonRpcMessage::Request { method, .. } => {
                tracing::debug!("received server request (unhandled): {}", method);
                Ok(())
            }
        }
    }

    /// Cancel a pending request. Returns true if it was found and canceled.
    pub fn cancel(&mut self, id: i64) -> bool {
        self.pending.remove(&id).is_some()
    }

    /// Cancel all pending requests.
    pub fn cancel_all(&mut self) {
        self.pending.clear();
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_new_empty() {
        let disp = Dispatcher::new();
        assert_eq!(disp.pending_count(), 0);
    }

    #[test]
    fn dispatcher_default_same_as_new() {
        let disp = Dispatcher::default();
        assert_eq!(disp.pending_count(), 0);
    }

    #[tokio::test]
    async fn dispatcher_register_and_resolve() {
        let mut disp = Dispatcher::new();
        let rx = disp.register_request(1);
        assert_eq!(disp.pending_count(), 1);

        let response = JsonRpcMessage::Response {
            id: 1,
            result: Some(serde_json::json!({"key": "value"})),
            error: None,
        };
        disp.dispatch(response).unwrap();
        assert_eq!(disp.pending_count(), 0);

        let result = rx.await.unwrap();
        match result {
            DispatchResult::Success(val) => assert_eq!(val["key"], "value"),
            DispatchResult::Error(_) => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn dispatcher_resolve_error() {
        let mut disp = Dispatcher::new();
        let rx = disp.register_request(1);

        let response = JsonRpcMessage::Response {
            id: 1,
            result: None,
            error: Some(RpcError {
                code: -32600,
                message: "invalid request".into(),
            }),
        };
        disp.dispatch(response).unwrap();

        let result = rx.await.unwrap();
        match result {
            DispatchResult::Error(err) => {
                assert_eq!(err.code, -32600);
                assert_eq!(err.message, "invalid request");
            }
            DispatchResult::Success(_) => panic!("expected error"),
        }
    }

    #[test]
    fn dispatcher_unknown_id_ignored() {
        let mut disp = Dispatcher::new();
        let response = JsonRpcMessage::Response {
            id: 999,
            result: Some(serde_json::json!(null)),
            error: None,
        };
        // Should not panic
        let result = disp.dispatch(response);
        assert!(result.is_ok());
    }

    #[test]
    fn dispatcher_notification_routed() {
        use std::sync::{Arc, Mutex};

        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let mut disp = Dispatcher::new();
        disp.set_notification_handler(Box::new(move |method, params| {
            received_clone.lock().unwrap().push((method, params));
        }));

        let notification = JsonRpcMessage::Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::json!({"uri": "file:///test.rs"}),
        };
        disp.dispatch(notification).unwrap();

        let captured = received.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, "textDocument/publishDiagnostics");
    }

    #[test]
    fn dispatcher_notification_without_handler() {
        let mut disp = Dispatcher::new();
        let notification = JsonRpcMessage::Notification {
            method: "test".into(),
            params: serde_json::json!(null),
        };
        // Should not panic
        assert!(disp.dispatch(notification).is_ok());
    }

    #[test]
    fn dispatcher_server_request_logged() {
        let mut disp = Dispatcher::new();
        let request = JsonRpcMessage::Request {
            id: 1,
            method: "window/showMessage".into(),
            params: serde_json::json!({}),
        };
        assert!(disp.dispatch(request).is_ok());
    }

    #[test]
    fn dispatcher_cancel_existing() {
        let mut disp = Dispatcher::new();
        let _rx = disp.register_request(1);
        assert_eq!(disp.pending_count(), 1);
        assert!(disp.cancel(1));
        assert_eq!(disp.pending_count(), 0);
    }

    #[test]
    fn dispatcher_cancel_nonexistent() {
        let mut disp = Dispatcher::new();
        assert!(!disp.cancel(999));
    }

    #[test]
    fn dispatcher_cancel_all() {
        let mut disp = Dispatcher::new();
        let _rx1 = disp.register_request(1);
        let _rx2 = disp.register_request(2);
        let _rx3 = disp.register_request(3);
        assert_eq!(disp.pending_count(), 3);
        disp.cancel_all();
        assert_eq!(disp.pending_count(), 0);
    }

    #[tokio::test]
    async fn dispatcher_multiple_concurrent_requests() {
        let mut disp = Dispatcher::new();
        let rx1 = disp.register_request(1);
        let rx2 = disp.register_request(2);
        let rx3 = disp.register_request(3);

        // Resolve in reverse order
        disp.dispatch(JsonRpcMessage::Response {
            id: 3,
            result: Some(serde_json::json!("third")),
            error: None,
        })
        .unwrap();
        disp.dispatch(JsonRpcMessage::Response {
            id: 1,
            result: Some(serde_json::json!("first")),
            error: None,
        })
        .unwrap();
        disp.dispatch(JsonRpcMessage::Response {
            id: 2,
            result: Some(serde_json::json!("second")),
            error: None,
        })
        .unwrap();

        match rx1.await.unwrap() {
            DispatchResult::Success(val) => assert_eq!(val, "first"),
            _ => panic!("expected success"),
        }
        match rx2.await.unwrap() {
            DispatchResult::Success(val) => assert_eq!(val, "second"),
            _ => panic!("expected success"),
        }
        match rx3.await.unwrap() {
            DispatchResult::Success(val) => assert_eq!(val, "third"),
            _ => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn dispatcher_dropped_receiver_doesnt_panic() {
        let mut disp = Dispatcher::new();
        let rx = disp.register_request(1);
        drop(rx);

        let response = JsonRpcMessage::Response {
            id: 1,
            result: Some(serde_json::json!(null)),
            error: None,
        };
        // Should not panic even though receiver is dropped
        assert!(disp.dispatch(response).is_ok());
    }

    #[tokio::test]
    async fn dispatcher_response_null_result() {
        let mut disp = Dispatcher::new();
        let rx = disp.register_request(1);

        disp.dispatch(JsonRpcMessage::Response {
            id: 1,
            result: None,
            error: None,
        })
        .unwrap();

        match rx.await.unwrap() {
            DispatchResult::Success(val) => assert!(val.is_null()),
            _ => panic!("expected success with null"),
        }
    }

    #[test]
    fn dispatch_result_debug() {
        let result = DispatchResult::Success(serde_json::json!("test"));
        let debug = format!("{:?}", result);
        assert!(debug.contains("Success"));
    }
}
