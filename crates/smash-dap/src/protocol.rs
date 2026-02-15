//! DAP protocol message types.
//!
//! Implements the Debug Adapter Protocol message structures with
//! serde Serialize/Deserialize support.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Base protocol messages
// ---------------------------------------------------------------------------

/// Base protocol message shared by all DAP messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Sequence number of this message.
    pub seq: i64,
    /// Message type: "request", "response", or "event".
    #[serde(rename = "type")]
    pub message_type: String,
}

/// A DAP request message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Sequence number.
    pub seq: i64,
    /// Always "request".
    #[serde(rename = "type")]
    pub message_type: String,
    /// The command to execute.
    pub command: String,
    /// Command arguments (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// A DAP response message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Sequence number.
    pub seq: i64,
    /// Always "response".
    #[serde(rename = "type")]
    pub message_type: String,
    /// Sequence number of the corresponding request.
    pub request_seq: i64,
    /// Whether the request was successful.
    pub success: bool,
    /// The command this response is for.
    pub command: String,
    /// Error message if `success` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response body (command-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// A DAP event message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Sequence number.
    pub seq: i64,
    /// Always "event".
    #[serde(rename = "type")]
    pub message_type: String,
    /// The event type.
    pub event: String,
    /// Event body (event-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Request arguments
// ---------------------------------------------------------------------------

/// Arguments for the `initialize` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArguments {
    /// ID of the client (e.g. "smash").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Human-readable name of the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    /// ID of the debug adapter.
    pub adapter_id: String,
    /// Client locale (e.g. "en-US").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Whether lines are 0-based. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_start_at1: Option<bool>,
    /// Whether columns are 0-based. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns_start_at1: Option<bool>,
    /// Path format: "path" or "uri".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_format: Option<String>,
    /// Whether the client supports variable type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_variable_type: Option<bool>,
    /// Whether the client supports variable paging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_variable_paging: Option<bool>,
    /// Whether the client supports the `runInTerminal` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_run_in_terminal_request: Option<bool>,
}

/// Capabilities returned by the debug adapter in the `initialize` response.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// The adapter supports the `configurationDone` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_configuration_done_request: Option<bool>,
    /// The adapter supports conditional breakpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_conditional_breakpoints: Option<bool>,
    /// The adapter supports hit conditional breakpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_hit_conditional_breakpoints: Option<bool>,
    /// The adapter supports `evaluate` for hovers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_evaluate_for_hovers: Option<bool>,
    /// The adapter supports stepping backwards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_back: Option<bool>,
    /// The adapter supports setting variable values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_variable: Option<bool>,
    /// The adapter supports the `terminate` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_request: Option<bool>,
}

/// Arguments for the `launch` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArguments {
    /// Whether to stop at entry point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_debug: Option<bool>,
    /// Restart data (for reconnect).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__restart")]
    pub restart: Option<serde_json::Value>,
    /// Program to launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    /// Command-line arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Working directory for the debuggee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Environment variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
    /// Stop at the entry point of the program.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_on_entry: Option<bool>,
}

/// Arguments for the `attach` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequestArguments {
    /// Restart data (for reconnect).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__restart")]
    pub restart: Option<serde_json::Value>,
    /// Process ID to attach to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<i64>,
}

/// Arguments for the `setBreakpoints` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArguments {
    /// The source to set breakpoints for.
    pub source: Source,
    /// Breakpoints to set (replaces all previous ones).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
}

/// Response body for `setBreakpoints`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsResponseBody {
    /// Information about the breakpoints.
    pub breakpoints: Vec<BreakpointResponse>,
}

/// A breakpoint as returned by the adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointResponse {
    /// Unique identifier for the breakpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Whether the breakpoint has been verified.
    pub verified: bool,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Actual source location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Actual line of the breakpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Actual column of the breakpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
}

/// A source breakpoint (client-side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    /// The source line of the breakpoint.
    pub line: i64,
    /// Optional column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Condition expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit condition expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    /// Log message (logpoint).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
}

// ---------------------------------------------------------------------------
// Step / flow-control arguments
// ---------------------------------------------------------------------------

/// Arguments for the `continue` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArguments {
    /// The thread to continue.
    pub thread_id: i64,
    /// Whether to continue just this thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_thread: Option<bool>,
}

/// Arguments for the `next` (step over) request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextArguments {
    /// The thread to step.
    pub thread_id: i64,
    /// Stepping granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
}

/// Arguments for the `stepIn` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInArguments {
    /// The thread to step.
    pub thread_id: i64,
    /// Target to step into (if multiple).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<i64>,
    /// Stepping granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
}

/// Arguments for the `stepOut` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutArguments {
    /// The thread to step.
    pub thread_id: i64,
    /// Stepping granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
}

/// Arguments for the `pause` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseArguments {
    /// The thread to pause.
    pub thread_id: i64,
}

// ---------------------------------------------------------------------------
// Runtime types
// ---------------------------------------------------------------------------

/// A thread in the debuggee.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// Unique identifier of the thread.
    pub id: i64,
    /// Human-readable name of the thread.
    pub name: String,
}

/// A stack frame in the call stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// Unique identifier for the stack frame.
    pub id: i64,
    /// Name of the frame (function name).
    pub name: String,
    /// Source location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line within the source.
    pub line: i64,
    /// Column within the source.
    pub column: i64,
    /// Module ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<serde_json::Value>,
}

/// A source location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Short name of the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// File system path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Source reference (for sources without a file path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
}

/// A scope (container for variables).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Name of the scope (e.g. "Locals", "Globals").
    pub name: String,
    /// Variables reference for this scope.
    pub variables_reference: i64,
    /// Whether the scope is expensive to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expensive: Option<bool>,
}

/// A variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// Name of the variable.
    pub name: String,
    /// Value of the variable as a string.
    pub value: String,
    /// Type of the variable.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_type: Option<String>,
    /// If > 0, the variable has children accessed via this reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables_reference: Option<i64>,
}

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

/// Arguments for the `evaluate` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArguments {
    /// The expression to evaluate.
    pub expression: String,
    /// Stack frame in whose context to evaluate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<i64>,
    /// Context: "watch", "repl", "hover", "clipboard".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Response body for `evaluate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponseBody {
    /// The result string.
    pub result: String,
    /// Type of the result.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    /// If > 0, the result has children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables_reference: Option<i64>,
}

// ---------------------------------------------------------------------------
// Disconnect
// ---------------------------------------------------------------------------

/// Arguments for the `disconnect` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectArguments {
    /// Whether to restart the debuggee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<bool>,
    /// Whether to terminate the debuggee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminate_debuggee: Option<bool>,
    /// Whether to suspend the debuggee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend_debuggee: Option<bool>,
}

// ---------------------------------------------------------------------------
// Event bodies
// ---------------------------------------------------------------------------

/// Reason why the debuggee stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    /// A step request completed.
    Step,
    /// A breakpoint was hit.
    Breakpoint,
    /// An exception occurred.
    Exception,
    /// A pause request was fulfilled.
    Pause,
    /// An entry point was reached.
    Entry,
    /// A goto request completed.
    Goto,
    /// A function breakpoint was hit.
    #[serde(rename = "function breakpoint")]
    FunctionBreakpoint,
    /// A data breakpoint was hit.
    #[serde(rename = "data breakpoint")]
    DataBreakpoint,
}

/// Body of the `stopped` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    /// The reason for the stop.
    pub reason: StopReason,
    /// Description of the stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Thread that stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<i64>,
    /// Whether all threads are stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_threads_stopped: Option<bool>,
    /// Additional text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Body of the `output` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
    /// Output category: "console", "stdout", "stderr", "telemetry".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// The output text.
    pub output: String,
    /// Source location that generated the output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line in the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Column in the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
}

/// Body of the `exited` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedEventBody {
    /// The exit code of the debuggee.
    pub exit_code: i64,
}

/// Body of the `terminated` event.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminatedEventBody {
    /// Restart data; if present, a restart is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_initialize_request_serde() {
        let args = InitializeRequestArguments {
            client_id: Some("smash".into()),
            client_name: Some("SMASH Editor".into()),
            adapter_id: "lldb".into(),
            locale: Some("en-US".into()),
            lines_start_at1: Some(true),
            columns_start_at1: Some(true),
            path_format: Some("path".into()),
            supports_variable_type: Some(true),
            supports_variable_paging: None,
            supports_run_in_terminal_request: None,
        };
        let json = serde_json::to_string(&args).unwrap();
        let decoded: InitializeRequestArguments = serde_json::from_str(&json).unwrap();
        assert_eq!(args, decoded);
    }

    #[test]
    fn protocol_launch_request_serde() {
        let args = LaunchRequestArguments {
            no_debug: Some(false),
            restart: None,
            program: Some("/usr/bin/myapp".into()),
            args: Some(vec!["--flag".into(), "value".into()]),
            cwd: Some("/home/user".into()),
            env: None,
            stop_on_entry: Some(true),
        };
        let json = serde_json::to_string(&args).unwrap();
        let decoded: LaunchRequestArguments = serde_json::from_str(&json).unwrap();
        assert_eq!(args, decoded);
    }

    #[test]
    fn protocol_stopped_event_serde() {
        let body = StoppedEventBody {
            reason: StopReason::Breakpoint,
            description: Some("Hit breakpoint 1".into()),
            thread_id: Some(1),
            all_threads_stopped: Some(true),
            text: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        let decoded: StoppedEventBody = serde_json::from_str(&json).unwrap();
        assert_eq!(body, decoded);
        assert!(json.contains("\"reason\":\"breakpoint\""));
    }

    #[test]
    fn protocol_breakpoint_serde() {
        let bp = SourceBreakpoint {
            line: 42,
            column: None,
            condition: Some("x > 10".into()),
            hit_condition: None,
            log_message: None,
        };
        let json = serde_json::to_string(&bp).unwrap();
        let decoded: SourceBreakpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(bp, decoded);
        assert!(json.contains("\"line\":42"));
    }

    #[test]
    fn protocol_stack_frame_serde() {
        let frame = StackFrame {
            id: 1,
            name: "main".into(),
            source: Some(Source {
                name: Some("main.rs".into()),
                path: Some("/src/main.rs".into()),
                source_reference: None,
            }),
            line: 10,
            column: 1,
            module_id: None,
        };
        let json = serde_json::to_string(&frame).unwrap();
        let decoded: StackFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(frame, decoded);
    }

    #[test]
    fn protocol_variable_serde() {
        let var = Variable {
            name: "counter".into(),
            value: "42".into(),
            variable_type: Some("i32".into()),
            variables_reference: Some(0),
        };
        let json = serde_json::to_string(&var).unwrap();
        let decoded: Variable = serde_json::from_str(&json).unwrap();
        assert_eq!(var, decoded);
    }

    #[test]
    fn protocol_request_serde() {
        let req = Request {
            seq: 1,
            message_type: "request".into(),
            command: "initialize".into(),
            arguments: Some(serde_json::json!({"adapterID": "lldb"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn protocol_response_serde() {
        let resp = Response {
            seq: 2,
            message_type: "response".into(),
            request_seq: 1,
            success: true,
            command: "initialize".into(),
            message: None,
            body: Some(serde_json::json!({})),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn protocol_event_serde() {
        let evt = Event {
            seq: 3,
            message_type: "event".into(),
            event: "stopped".into(),
            body: Some(serde_json::json!({"reason": "breakpoint", "threadId": 1})),
        };
        let json = serde_json::to_string(&evt).unwrap();
        let decoded: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(evt, decoded);
    }

    #[test]
    fn protocol_capabilities_serde() {
        let caps = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_conditional_breakpoints: Some(true),
            supports_hit_conditional_breakpoints: None,
            supports_evaluate_for_hovers: Some(false),
            supports_step_back: None,
            supports_set_variable: None,
            supports_terminate_request: Some(true),
        };
        let json = serde_json::to_string(&caps).unwrap();
        let decoded: Capabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps, decoded);
    }

    #[test]
    fn protocol_evaluate_serde() {
        let args = EvaluateArguments {
            expression: "x + y".into(),
            frame_id: Some(1),
            context: Some("repl".into()),
        };
        let json = serde_json::to_string(&args).unwrap();
        let decoded: EvaluateArguments = serde_json::from_str(&json).unwrap();
        assert_eq!(args, decoded);

        let body = EvaluateResponseBody {
            result: "42".into(),
            result_type: Some("int".into()),
            variables_reference: Some(0),
        };
        let json = serde_json::to_string(&body).unwrap();
        let decoded: EvaluateResponseBody = serde_json::from_str(&json).unwrap();
        assert_eq!(body, decoded);
    }

    #[test]
    fn protocol_disconnect_serde() {
        let args = DisconnectArguments {
            restart: Some(false),
            terminate_debuggee: Some(true),
            suspend_debuggee: None,
        };
        let json = serde_json::to_string(&args).unwrap();
        let decoded: DisconnectArguments = serde_json::from_str(&json).unwrap();
        assert_eq!(args, decoded);
    }

    #[test]
    fn protocol_stop_reason_variants() {
        let reasons = vec![
            (StopReason::Step, "\"step\""),
            (StopReason::Breakpoint, "\"breakpoint\""),
            (StopReason::Exception, "\"exception\""),
            (StopReason::Pause, "\"pause\""),
            (StopReason::Entry, "\"entry\""),
            (StopReason::Goto, "\"goto\""),
            (StopReason::FunctionBreakpoint, "\"function breakpoint\""),
            (StopReason::DataBreakpoint, "\"data breakpoint\""),
        ];
        for (reason, expected_json) in reasons {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, expected_json);
            let decoded: StopReason = serde_json::from_str(&json).unwrap();
            assert_eq!(reason, decoded);
        }
    }
}
