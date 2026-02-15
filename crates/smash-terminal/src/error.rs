/// Errors that can occur in the terminal emulator.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    /// PTY creation failed.
    #[error("PTY creation failed: {0}")]
    PtyFailed(String),

    /// Shell process failed to start.
    #[error("shell process failed to start: {0}")]
    ShellSpawnFailed(#[from] std::io::Error),

    /// Terminal I/O error.
    #[error("terminal I/O error: {0}")]
    Io(String),

    /// Terminal process exited.
    #[error("terminal process exited: {0}")]
    Exited(i32),

    /// Resize failed.
    #[error("resize failed: {0}")]
    ResizeFailed(String),
}

/// Result type alias for terminal operations.
pub type TerminalResult<T> = Result<T, TerminalError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_pty_failed() {
        let err = TerminalError::PtyFailed("no pty available".to_string());
        assert_eq!(err.to_string(), "PTY creation failed: no pty available");
    }

    #[test]
    fn error_display_io() {
        let err = TerminalError::Io("broken pipe".to_string());
        assert_eq!(err.to_string(), "terminal I/O error: broken pipe");
    }

    #[test]
    fn error_display_exited() {
        let err = TerminalError::Exited(1);
        assert_eq!(err.to_string(), "terminal process exited: 1");
    }

    #[test]
    fn error_display_resize_failed() {
        let err = TerminalError::ResizeFailed("invalid size".to_string());
        assert_eq!(err.to_string(), "resize failed: invalid size");
    }

    #[test]
    fn error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "shell not found");
        let err = TerminalError::from(io_err);
        assert!(matches!(err, TerminalError::ShellSpawnFailed(_)));
    }

    #[test]
    fn error_is_debug() {
        let err = TerminalError::PtyFailed("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("PtyFailed"));
    }
}
