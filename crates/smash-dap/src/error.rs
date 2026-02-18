//! DAP error types.

use thiserror::Error;

/// Errors from DAP client operations.
#[derive(Debug, Error)]
pub enum DapError {
    /// Adapter process failed to start.
    #[error("adapter failed to start: {0}")]
    AdapterSpawnFailed(#[from] std::io::Error),

    /// Transport-level communication error.
    #[error("transport error: {0}")]
    Transport(String),

    /// Request timed out waiting for a response.
    #[error("request timed out: {command}")]
    Timeout {
        /// The command that timed out.
        command: String,
    },

    /// Adapter rejected the request.
    #[error("adapter rejected request: {message}")]
    Rejected {
        /// The rejection message from the adapter.
        message: String,
    },

    /// Adapter sent an invalid or unparseable response.
    #[error("adapter sent invalid response: {0}")]
    InvalidResponse(String),

    /// Session has not been initialized yet.
    #[error("session not initialized")]
    NotInitialized,

    /// Session has already been terminated.
    #[error("session already terminated")]
    Terminated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_adapter_spawn_failed_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "binary missing");
        let err = DapError::AdapterSpawnFailed(io_err);
        assert!(err.to_string().contains("adapter failed to start"));
        assert!(err.to_string().contains("binary missing"));
    }

    #[test]
    fn error_transport_display() {
        let err = DapError::Transport("connection reset".into());
        assert_eq!(err.to_string(), "transport error: connection reset");
    }

    #[test]
    fn error_timeout_display() {
        let err = DapError::Timeout {
            command: "evaluate".into(),
        };
        assert_eq!(err.to_string(), "request timed out: evaluate");
    }

    #[test]
    fn error_rejected_display() {
        let err = DapError::Rejected {
            message: "not supported".into(),
        };
        assert_eq!(err.to_string(), "adapter rejected request: not supported");
    }

    #[test]
    fn error_invalid_response_display() {
        let err = DapError::InvalidResponse("unexpected null".into());
        assert_eq!(
            err.to_string(),
            "adapter sent invalid response: unexpected null"
        );
    }

    #[test]
    fn error_not_initialized_display() {
        let err = DapError::NotInitialized;
        assert_eq!(err.to_string(), "session not initialized");
    }

    #[test]
    fn error_terminated_display() {
        let err = DapError::Terminated;
        assert_eq!(err.to_string(), "session already terminated");
    }

    #[test]
    fn error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let err: DapError = io_err.into();
        assert!(matches!(err, DapError::AdapterSpawnFailed(_)));
    }
}
