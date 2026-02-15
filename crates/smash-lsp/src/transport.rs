//! JSON-RPC transport for LSP communication.
//!
//! Implements Content-Length header framing per the LSP specification.
use std::sync::atomic::{AtomicI64, Ordering};

use crate::error::LspError;

/// Global request ID counter.
static NEXT_REQUEST_ID: AtomicI64 = AtomicI64::new(1);

/// Generate the next unique request ID.
pub fn next_request_id() -> i64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// A JSON-RPC message (request, response, or notification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRpcMessage {
    /// A request (has id and method).
    Request {
        /// The request ID.
        id: i64,
        /// The method name.
        method: String,
        /// The params (JSON value).
        params: serde_json::Value,
    },
    /// A response (has id, may have result or error).
    Response {
        /// The request ID this responds to.
        id: i64,
        /// The result (if successful).
        result: Option<serde_json::Value>,
        /// The error (if failed).
        error: Option<RpcError>,
    },
    /// A notification (has method, no id).
    Notification {
        /// The method name.
        method: String,
        /// The params.
        params: serde_json::Value,
    },
}

/// An error object in a JSON-RPC response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    /// The error code.
    pub code: i32,
    /// The error message.
    pub message: String,
}

/// Frame a JSON-RPC message with Content-Length header.
pub fn frame_message(body: &str) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

/// Serialize a JSON-RPC request.
pub fn serialize_request(id: i64, method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
    .to_string()
}

/// Serialize a JSON-RPC notification (no id).
pub fn serialize_notification(method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
    .to_string()
}

/// Serialize a JSON-RPC response.
pub fn serialize_response(id: i64, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
    .to_string()
}

/// Serialize a JSON-RPC error response.
pub fn serialize_error_response(id: i64, code: i32, message: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
    .to_string()
}

/// Parse the Content-Length value from raw header bytes.
///
/// Returns the body length if the header is valid.
pub fn parse_content_length(header: &str) -> Result<usize, LspError> {
    for line in header.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let value = value.trim();
            return value.parse::<usize>().map_err(|_| {
                LspError::InvalidResponse(format!("invalid Content-Length: {}", value))
            });
        }
    }
    Err(LspError::InvalidResponse(
        "missing Content-Length header".to_string(),
    ))
}

/// Parse a JSON-RPC message from a JSON string.
pub fn parse_message(json_str: &str) -> Result<JsonRpcMessage, LspError> {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| LspError::Serialization(format!("invalid JSON: {}", e)))?;

    let has_id = value.get("id").is_some();
    let has_method = value.get("method").is_some();

    match (has_id, has_method) {
        // Request: has both id and method
        (true, true) => {
            let id = value["id"]
                .as_i64()
                .ok_or_else(|| LspError::InvalidResponse("id must be integer".into()))?;
            let method = value["method"]
                .as_str()
                .ok_or_else(|| LspError::InvalidResponse("method must be string".into()))?
                .to_string();
            let params = value
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Ok(JsonRpcMessage::Request { id, method, params })
        }
        // Response: has id but no method
        (true, false) => {
            let id = value["id"]
                .as_i64()
                .ok_or_else(|| LspError::InvalidResponse("id must be integer".into()))?;
            let result = value.get("result").cloned();
            let error = value.get("error").and_then(|e| {
                Some(RpcError {
                    code: e.get("code")?.as_i64()? as i32,
                    message: e.get("message")?.as_str()?.to_string(),
                })
            });
            Ok(JsonRpcMessage::Response { id, result, error })
        }
        // Notification: has method but no id
        (false, true) => {
            let method = value["method"]
                .as_str()
                .ok_or_else(|| LspError::InvalidResponse("method must be string".into()))?
                .to_string();
            let params = value
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            Ok(JsonRpcMessage::Notification { method, params })
        }
        // Invalid
        (false, false) => Err(LspError::InvalidResponse(
            "message has neither id nor method".to_string(),
        )),
    }
}

/// Read a single LSP message from raw bytes (header + body).
///
/// The input should contain the full Content-Length header and body.
/// Returns the parsed message and the number of bytes consumed.
pub fn read_message_from_bytes(input: &[u8]) -> Result<(JsonRpcMessage, usize), LspError> {
    let input_str = std::str::from_utf8(input)
        .map_err(|_| LspError::InvalidResponse("invalid UTF-8 in input".into()))?;

    let header_end = input_str
        .find("\r\n\r\n")
        .ok_or_else(|| LspError::InvalidResponse("incomplete header".into()))?;

    let header = &input_str[..header_end];
    let content_length = parse_content_length(header)?;

    let body_start = header_end + 4; // Skip \r\n\r\n
    let body_end = body_start + content_length;

    if input_str.len() < body_end {
        return Err(LspError::InvalidResponse(format!(
            "incomplete body: expected {} bytes, got {}",
            content_length,
            input_str.len() - body_start
        )));
    }

    let body = &input_str[body_start..body_end];
    let msg = parse_message(body)?;
    Ok((msg, body_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_request_id_increments() {
        let a = next_request_id();
        let b = next_request_id();
        assert!(b > a);
    }

    #[test]
    fn frame_message_format() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let framed = frame_message(body);
        let framed_str = String::from_utf8(framed).unwrap();
        assert!(framed_str.starts_with("Content-Length: "));
        assert!(framed_str.contains("\r\n\r\n"));
        assert!(framed_str.ends_with(body));
    }

    #[test]
    fn frame_message_correct_length() {
        let body = "hello world";
        let framed = frame_message(body);
        let framed_str = String::from_utf8(framed).unwrap();
        assert!(framed_str.contains("Content-Length: 11\r\n\r\n"));
    }

    #[test]
    fn serialize_request_format() {
        let json = serialize_request(1, "initialize", serde_json::json!({}));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 1);
        assert_eq!(value["method"], "initialize");
    }

    #[test]
    fn serialize_notification_no_id() {
        let json = serialize_notification("initialized", serde_json::json!({}));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["method"], "initialized");
        assert!(value.get("id").is_none());
    }

    #[test]
    fn serialize_response_format() {
        let json = serialize_response(1, serde_json::json!({"key": "value"}));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 1);
        assert_eq!(value["result"]["key"], "value");
    }

    #[test]
    fn serialize_error_response_format() {
        let json = serialize_error_response(1, -32600, "invalid request");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["error"]["code"], -32600);
        assert_eq!(value["error"]["message"], "invalid request");
    }

    #[test]
    fn parse_content_length_valid() {
        let header = "Content-Length: 42";
        assert_eq!(parse_content_length(header).unwrap(), 42);
    }

    #[test]
    fn parse_content_length_with_extra_headers() {
        let header = "Content-Type: application/json\r\nContent-Length: 100";
        assert_eq!(parse_content_length(header).unwrap(), 100);
    }

    #[test]
    fn parse_content_length_missing() {
        let header = "Content-Type: application/json";
        assert!(parse_content_length(header).is_err());
    }

    #[test]
    fn parse_content_length_invalid_number() {
        let header = "Content-Length: abc";
        assert!(parse_content_length(header).is_err());
    }

    #[test]
    fn parse_message_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Request { id, method, .. } => {
                assert_eq!(id, 1);
                assert_eq!(method, "initialize");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn parse_message_response_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Response {
                id, result, error, ..
            } => {
                assert_eq!(id, 1);
                assert!(result.is_some());
                assert!(error.is_none());
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn parse_message_response_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"invalid request"}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Response { id, error, .. } => {
                assert_eq!(id, 1);
                let err = error.unwrap();
                assert_eq!(err.code, -32600);
                assert_eq!(err.message, "invalid request");
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn parse_message_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///test.rs","diagnostics":[]}}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Notification { method, params } => {
                assert_eq!(method, "textDocument/publishDiagnostics");
                assert!(params["uri"].as_str().is_some());
            }
            _ => panic!("expected notification"),
        }
    }

    #[test]
    fn parse_message_invalid_json() {
        let result = parse_message("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_message_no_id_no_method() {
        let json = r#"{"jsonrpc":"2.0"}"#;
        let result = parse_message(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_message_request_without_params() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#;
        let msg = parse_message(json).unwrap();
        match msg {
            JsonRpcMessage::Request { params, .. } => {
                assert!(params.is_null());
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn read_message_from_bytes_valid() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let framed = frame_message(body);
        let (msg, consumed) = read_message_from_bytes(&framed).unwrap();
        assert_eq!(consumed, framed.len());
        match msg {
            JsonRpcMessage::Request { id, method, .. } => {
                assert_eq!(id, 1);
                assert_eq!(method, "test");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn read_message_from_bytes_incomplete_header() {
        let input = b"Content-Length: 10";
        let result = read_message_from_bytes(input);
        assert!(result.is_err());
    }

    #[test]
    fn read_message_from_bytes_incomplete_body() {
        let input = b"Content-Length: 100\r\n\r\nshort";
        let result = read_message_from_bytes(input);
        assert!(result.is_err());
    }

    #[test]
    fn read_message_from_bytes_multiple_messages() {
        let body1 = r#"{"jsonrpc":"2.0","id":1,"method":"a"}"#;
        let body2 = r#"{"jsonrpc":"2.0","id":2,"method":"b"}"#;
        let mut data = frame_message(body1);
        data.extend_from_slice(&frame_message(body2));

        let (msg1, consumed1) = read_message_from_bytes(&data).unwrap();
        let (msg2, _consumed2) = read_message_from_bytes(&data[consumed1..]).unwrap();

        match msg1 {
            JsonRpcMessage::Request { id, .. } => assert_eq!(id, 1),
            _ => panic!("expected request"),
        }
        match msg2 {
            JsonRpcMessage::Request { id, .. } => assert_eq!(id, 2),
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn rpc_error_debug() {
        let err = RpcError {
            code: -32600,
            message: "invalid".into(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("RpcError"));
    }

    #[test]
    fn json_rpc_message_clone() {
        let msg = JsonRpcMessage::Notification {
            method: "test".into(),
            params: serde_json::json!({}),
        };
        let cloned = msg.clone();
        assert_eq!(cloned, msg);
    }
}
