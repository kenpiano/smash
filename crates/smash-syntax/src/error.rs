use thiserror::Error;

/// Errors that can occur during syntax highlighting operations.
#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("unknown language: {0}")]
    UnknownLanguage(String),
    #[error("invalid regex pattern in {language} rule: {detail}")]
    InvalidPattern { language: String, detail: String },
    #[error("highlight engine error: {0}")]
    EngineError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_language_display() {
        let err = SyntaxError::UnknownLanguage("brainfuck".to_string());
        assert_eq!(err.to_string(), "unknown language: brainfuck");
    }

    #[test]
    fn invalid_pattern_display() {
        let err = SyntaxError::InvalidPattern {
            language: "rust".to_string(),
            detail: "unclosed group".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid regex pattern in rust rule: unclosed group"
        );
    }

    #[test]
    fn engine_error_display() {
        let err = SyntaxError::EngineError("out of memory".to_string());
        assert_eq!(err.to_string(), "highlight engine error: out of memory");
    }

    #[test]
    fn error_is_debug() {
        let err = SyntaxError::UnknownLanguage("x".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("UnknownLanguage"));
    }
}
