use thiserror::Error;

/// Errors that can occur during platform operations.
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("clipboard operation failed: {0}")]
    Clipboard(String),

    #[error("path error: {0}")]
    Path(String),

    #[error("process spawn failed: {0}")]
    ProcessSpawn(#[from] std::io::Error),

    #[error("signal handler error: {0}")]
    Signal(String),

    #[error("unsupported operation on {os}: {detail}")]
    Unsupported { os: String, detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_error_display_contains_message() {
        let err = PlatformError::Clipboard("test failure".into());
        assert!(err.to_string().contains("clipboard operation failed"));
        assert!(err.to_string().contains("test failure"));
    }

    #[test]
    fn path_error_display_contains_message() {
        let err = PlatformError::Path("bad path".into());
        assert!(err.to_string().contains("path error"));
        assert!(err.to_string().contains("bad path"));
    }

    #[test]
    fn unsupported_error_display_contains_os_and_detail() {
        let err = PlatformError::Unsupported {
            os: "windows".into(),
            detail: "not implemented".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("windows"));
        assert!(msg.contains("not implemented"));
    }

    #[test]
    fn process_spawn_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "cmd not found");
        let err = PlatformError::from(io_err);
        assert!(err.to_string().contains("cmd not found"));
    }

    #[test]
    fn signal_error_display_contains_message() {
        let err = PlatformError::Signal("SIGTERM".into());
        assert!(err.to_string().contains("signal handler error"));
        assert!(err.to_string().contains("SIGTERM"));
    }
}
