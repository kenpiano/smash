//! LSP error types.
/// Errors from LSP client operations.
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    /// Server process failed to start.
    #[error("server failed to start: {0}")]
    SpawnFailed(String),

    /// Server initialization handshake failed.
    #[error("server initialization failed: {0}")]
    InitFailed(String),

    /// JSON-RPC error returned by the server.
    #[error("JSON-RPC error {code}: {message}")]
    Rpc {
        /// The error code.
        code: i32,
        /// The error message.
        message: String,
    },

    /// Request timed out waiting for a response.
    #[error("request timed out after {0} seconds")]
    Timeout(u64),

    /// Server process exited unexpectedly.
    #[error("server process exited unexpectedly")]
    ServerCrashed,

    /// Serialization or deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Server not found in registry.
    #[error("no server registered for language: {0}")]
    NoServer(String),

    /// Server already running.
    #[error("server already running for language: {0}")]
    AlreadyRunning(String),

    /// Invalid response from server.
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_spawn_failed_display() {
        let err = LspError::SpawnFailed("not found".into());
        assert_eq!(err.to_string(), "server failed to start: not found");
    }

    #[test]
    fn error_init_failed_display() {
        let err = LspError::InitFailed("bad caps".into());
        assert_eq!(err.to_string(), "server initialization failed: bad caps");
    }

    #[test]
    fn error_rpc_display() {
        let err = LspError::Rpc {
            code: -32600,
            message: "invalid request".into(),
        };
        assert_eq!(err.to_string(), "JSON-RPC error -32600: invalid request");
    }

    #[test]
    fn error_timeout_display() {
        let err = LspError::Timeout(10);
        assert_eq!(err.to_string(), "request timed out after 10 seconds");
    }

    #[test]
    fn error_server_crashed_display() {
        let err = LspError::ServerCrashed;
        assert_eq!(err.to_string(), "server process exited unexpectedly");
    }

    #[test]
    fn error_serialization_display() {
        let err = LspError::Serialization("bad json".into());
        assert_eq!(err.to_string(), "serialization error: bad json");
    }

    #[test]
    fn error_io_from() {
        let io = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let err = LspError::from(io);
        assert!(err.to_string().contains("broken"));
    }

    #[test]
    fn error_no_server_display() {
        let err = LspError::NoServer("rust".into());
        assert_eq!(err.to_string(), "no server registered for language: rust");
    }

    #[test]
    fn error_already_running_display() {
        let err = LspError::AlreadyRunning("python".into());
        assert_eq!(
            err.to_string(),
            "server already running for language: python"
        );
    }

    #[test]
    fn error_invalid_response_display() {
        let err = LspError::InvalidResponse("missing result".into());
        assert_eq!(err.to_string(), "invalid response: missing result");
    }

    #[test]
    fn error_is_debug() {
        let err = LspError::Timeout(5);
        let debug = format!("{:?}", err);
        assert!(debug.contains("Timeout"));
    }
}
