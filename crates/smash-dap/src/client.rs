//! High-level DAP client.

use crate::breakpoint::BreakpointManager;
use crate::error::DapError;
use crate::protocol::{
    ContinueArguments, DisconnectArguments, EvaluateArguments, NextArguments, PauseArguments,
    Request, SetBreakpointsArguments, Source, SourceBreakpoint, StepInArguments, StepOutArguments,
};
use crate::session::{DapSession, SessionState};

use std::path::Path;

/// A high-level DAP client that wraps session management, breakpoint
/// tracking, and request construction.
#[derive(Debug)]
pub struct DapClient {
    session: DapSession,
    breakpoints: BreakpointManager,
}

impl DapClient {
    /// Create a new DAP client.
    pub fn new() -> Self {
        Self {
            session: DapSession::new(),
            breakpoints: BreakpointManager::new(),
        }
    }

    /// Return a reference to the underlying session.
    pub fn session(&self) -> &DapSession {
        &self.session
    }

    /// Return a mutable reference to the underlying session.
    pub fn session_mut(&mut self) -> &mut DapSession {
        &mut self.session
    }

    /// Return a reference to the breakpoint manager.
    pub fn breakpoints(&self) -> &BreakpointManager {
        &self.breakpoints
    }

    /// Return a mutable reference to the breakpoint manager.
    pub fn breakpoints_mut(&mut self) -> &mut BreakpointManager {
        &mut self.breakpoints
    }

    /// Require the session to be in the Stopped state (or at least running)
    /// for stepping / inspection operations.
    fn require_stopped(&self) -> Result<(), DapError> {
        match self.session.state() {
            SessionState::Uninitialized => Err(DapError::NotInitialized),
            SessionState::Terminated => Err(DapError::Terminated),
            SessionState::Stopped => Ok(()),
            other => Err(DapError::Rejected {
                message: format!("operation requires Stopped state, currently {:?}", other),
            }),
        }
    }

    /// Require the session to be at least initialized (not uninitialized
    /// or terminated).
    fn require_at_least_initialized(&self) -> Result<(), DapError> {
        match self.session.state() {
            SessionState::Uninitialized => Err(DapError::NotInitialized),
            SessionState::Terminated => Err(DapError::Terminated),
            _ => Ok(()),
        }
    }

    /// Build a `next` (step over) request for the given thread.
    pub fn step_over(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        let args = NextArguments {
            thread_id,
            granularity: None,
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "next".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `stepIn` request for the given thread.
    pub fn step_into(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        let args = StepInArguments {
            thread_id,
            target_id: None,
            granularity: None,
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "stepIn".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `stepOut` request for the given thread.
    pub fn step_out(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        let args = StepOutArguments {
            thread_id,
            granularity: None,
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "stepOut".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `threads` request.
    pub fn threads(&mut self) -> Result<Request, DapError> {
        self.require_at_least_initialized()?;
        let seq = self.session.next_seq();
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "threads".into(),
            arguments: None,
        })
    }

    /// Build a `stackTrace` request for the given thread.
    pub fn stack_trace(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "stackTrace".into(),
            arguments: Some(serde_json::json!({
                "threadId": thread_id,
            })),
        })
    }

    /// Build a `scopes` request for the given frame.
    pub fn scopes(&mut self, frame_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "scopes".into(),
            arguments: Some(serde_json::json!({
                "frameId": frame_id,
            })),
        })
    }

    /// Build a `variables` request for the given variables reference.
    pub fn variables(&mut self, variables_reference: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "variables".into(),
            arguments: Some(serde_json::json!({
                "variablesReference": variables_reference,
            })),
        })
    }

    /// Build an `evaluate` request.
    pub fn evaluate(
        &mut self,
        expression: &str,
        frame_id: Option<i64>,
        context: Option<&str>,
    ) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        let args = EvaluateArguments {
            expression: expression.into(),
            frame_id,
            context: context.map(|c| c.into()),
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "evaluate".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `continue` request for the given thread.
    pub fn continue_request(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_stopped()?;
        let seq = self.session.next_seq();
        let args = ContinueArguments {
            thread_id,
            single_thread: None,
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "continue".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `pause` request for the given thread.
    pub fn pause(&mut self, thread_id: i64) -> Result<Request, DapError> {
        self.require_at_least_initialized()?;
        let seq = self.session.next_seq();
        let args = PauseArguments { thread_id };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "pause".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `setBreakpoints` request for the given file, using
    /// the breakpoints currently tracked by the manager.
    pub fn set_breakpoints_for_file(&mut self, path: &Path) -> Result<Request, DapError> {
        self.require_at_least_initialized()?;
        let seq = self.session.next_seq();
        let bps: Vec<SourceBreakpoint> = self
            .breakpoints
            .get_for_file(path)
            .iter()
            .map(|bp| SourceBreakpoint {
                line: bp.line,
                column: None,
                condition: bp.condition.clone(),
                hit_condition: bp.hit_condition.clone(),
                log_message: bp.log_message.clone(),
            })
            .collect();

        let args = SetBreakpointsArguments {
            source: Source {
                name: path.file_name().map(|n| n.to_string_lossy().into_owned()),
                path: Some(path.to_string_lossy().into_owned()),
                source_reference: None,
            },
            breakpoints: Some(bps),
        };

        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "setBreakpoints".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }

    /// Build a `disconnect` request.
    pub fn disconnect(&mut self, terminate_debuggee: Option<bool>) -> Result<Request, DapError> {
        // disconnect is valid from any non-terminated state
        if self.session.state() == SessionState::Terminated {
            return Err(DapError::Terminated);
        }
        let seq = self.session.next_seq();
        let args = DisconnectArguments {
            restart: Some(false),
            terminate_debuggee,
            suspend_debuggee: None,
        };
        Ok(Request {
            seq,
            message_type: "request".into(),
            command: "disconnect".into(),
            arguments: Some(
                serde_json::to_value(args).map_err(|e| DapError::Transport(e.to_string()))?,
            ),
        })
    }
}

impl Default for DapClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breakpoint::Breakpoint;
    use crate::protocol::Capabilities;
    use std::path::PathBuf;

    /// Helper: create a client in the Stopped state.
    fn stopped_client() -> DapClient {
        let mut client = DapClient::new();
        let caps = Capabilities {
            supports_configuration_done_request: Some(true),
            ..Default::default()
        };
        client.session_mut().initialize(&caps).unwrap();
        client.session_mut().launch().unwrap();
        client.session_mut().handle_stopped().unwrap();
        client
    }

    #[test]
    fn client_step_over() {
        let mut client = stopped_client();
        let req = client.step_over(1).unwrap();
        assert_eq!(req.command, "next");
        assert_eq!(req.message_type, "request");

        let args: serde_json::Value = req.arguments.unwrap();
        assert_eq!(args["threadId"], 1);
    }

    #[test]
    fn client_step_into() {
        let mut client = stopped_client();
        let req = client.step_into(1).unwrap();
        assert_eq!(req.command, "stepIn");

        let args = req.arguments.unwrap();
        assert_eq!(args["threadId"], 1);
    }

    #[test]
    fn client_step_out() {
        let mut client = stopped_client();
        let req = client.step_out(1).unwrap();
        assert_eq!(req.command, "stepOut");
    }

    #[test]
    fn client_threads() {
        let mut client = stopped_client();
        let req = client.threads().unwrap();
        assert_eq!(req.command, "threads");
        assert!(req.arguments.is_none());
    }

    #[test]
    fn client_stack_trace() {
        let mut client = stopped_client();
        let req = client.stack_trace(1).unwrap();
        assert_eq!(req.command, "stackTrace");

        let args = req.arguments.unwrap();
        assert_eq!(args["threadId"], 1);
    }

    #[test]
    fn client_scopes_and_variables() {
        let mut client = stopped_client();

        let req = client.scopes(0).unwrap();
        assert_eq!(req.command, "scopes");
        let args = req.arguments.unwrap();
        assert_eq!(args["frameId"], 0);

        let req = client.variables(100).unwrap();
        assert_eq!(req.command, "variables");
        let args = req.arguments.unwrap();
        assert_eq!(args["variablesReference"], 100);
    }

    #[test]
    fn client_evaluate() {
        let mut client = stopped_client();
        let req = client.evaluate("x + y", Some(0), Some("repl")).unwrap();
        assert_eq!(req.command, "evaluate");

        let args = req.arguments.unwrap();
        assert_eq!(args["expression"], "x + y");
        assert_eq!(args["frameId"], 0);
        assert_eq!(args["context"], "repl");
    }

    #[test]
    fn client_step_over_before_init_fails() {
        let mut client = DapClient::new();
        let err = client.step_over(1).unwrap_err();
        assert!(matches!(err, DapError::NotInitialized));
    }

    #[test]
    fn client_step_over_when_running_fails() {
        let mut client = DapClient::new();
        let caps = Capabilities::default();
        client.session_mut().initialize(&caps).unwrap();
        client.session_mut().launch().unwrap();
        // State is Running, not Stopped.
        let err = client.step_over(1).unwrap_err();
        assert!(matches!(err, DapError::Rejected { .. }));
    }

    #[test]
    fn client_disconnect_builds_request() {
        let mut client = stopped_client();
        let req = client.disconnect(Some(true)).unwrap();
        assert_eq!(req.command, "disconnect");

        let args = req.arguments.unwrap();
        assert_eq!(args["terminateDebuggee"], true);
    }

    #[test]
    fn client_break_and_set_breakpoints() {
        let mut client = stopped_client();
        let path = PathBuf::from("/src/main.rs");

        client
            .breakpoints_mut()
            .add(Breakpoint::new(path.clone(), 10));
        client
            .breakpoints_mut()
            .add(Breakpoint::new(path.clone(), 20));

        let req = client.set_breakpoints_for_file(&path).unwrap();
        assert_eq!(req.command, "setBreakpoints");

        let args = req.arguments.unwrap();
        let bps = args["breakpoints"].as_array().unwrap();
        assert_eq!(bps.len(), 2);
        assert_eq!(bps[0]["line"], 10);
        assert_eq!(bps[1]["line"], 20);
    }

    #[test]
    fn client_continue_request() {
        let mut client = stopped_client();
        let req = client.continue_request(1).unwrap();
        assert_eq!(req.command, "continue");

        let args = req.arguments.unwrap();
        assert_eq!(args["threadId"], 1);
    }

    #[test]
    fn client_pause_when_running() {
        let mut client = DapClient::new();
        let caps = Capabilities::default();
        client.session_mut().initialize(&caps).unwrap();
        client.session_mut().launch().unwrap();

        let req = client.pause(1).unwrap();
        assert_eq!(req.command, "pause");
    }

    #[test]
    fn client_sequence_numbers_increment() {
        let mut client = stopped_client();
        let r1 = client.step_over(1).unwrap();
        let r2 = client.step_over(1).unwrap();
        assert!(r2.seq > r1.seq);
    }
}
