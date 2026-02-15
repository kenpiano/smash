use thiserror::Error;

#[derive(Debug, Error)]
pub enum TuiError {
    #[error("terminal I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pane layout: {0}")]
    Layout(String),
    #[error("theme error: {0}")]
    Theme(String),
    #[error("render error: {0}")]
    Render(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_io_display() {
        let io_err = std::io::Error::other("disk full");
        let err = TuiError::from(io_err);
        assert_eq!(err.to_string(), "terminal I/O error: disk full");
    }

    #[test]
    fn error_layout_display() {
        let err = TuiError::Layout("bad split".into());
        assert_eq!(err.to_string(), "invalid pane layout: bad split",);
    }

    #[test]
    fn error_theme_display() {
        let err = TuiError::Theme("missing scope".into());
        assert_eq!(err.to_string(), "theme error: missing scope");
    }

    #[test]
    fn error_render_display() {
        let err = TuiError::Render("out of bounds".into());
        assert_eq!(err.to_string(), "render error: out of bounds",);
    }

    #[test]
    fn error_io_from_conversion() {
        let io_err = std::io::Error::other("not found");
        let err: TuiError = io_err.into();
        assert!(matches!(err, TuiError::Io(_)));
    }

    #[test]
    fn error_is_debug() {
        let err = TuiError::Layout("test".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Layout"));
    }
}
