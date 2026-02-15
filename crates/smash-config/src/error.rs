use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration loading, parsing,
/// or validation.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The specified config file was not found.
    #[error("config file not found: {0}")]
    NotFound(PathBuf),

    /// Failed to create the default config file.
    #[error("failed to create default config: {0}")]
    CreateDefault(String),

    /// TOML parsing failed.
    #[error("TOML parse error: {0}")]
    Parse(String),

    /// A config value failed validation.
    #[error("validation error: {field}: {message}")]
    Validation {
        /// The dotted field path (e.g. `editor.tab_size`).
        field: String,
        /// Human-readable description of the violation.
        message: String,
    },

    /// An I/O error occurred while reading or writing config files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display_contains_path() {
        let err = ConfigError::NotFound(PathBuf::from("/tmp/missing.toml"));
        let msg = format!("{err}");
        assert!(msg.contains("/tmp/missing.toml"));
        assert!(msg.contains("config file not found"));
    }

    #[test]
    fn create_default_display_contains_reason() {
        let err = ConfigError::CreateDefault("permission denied".into());
        let msg = format!("{err}");
        assert!(msg.contains("permission denied"));
        assert!(msg.contains("failed to create default config"));
    }

    #[test]
    fn parse_display_contains_details() {
        let err = ConfigError::Parse("unexpected `=`".into());
        let msg = format!("{err}");
        assert!(msg.contains("unexpected `=`"));
        assert!(msg.contains("TOML parse error"));
    }

    #[test]
    fn validation_display_contains_field_and_message() {
        let err = ConfigError::Validation {
            field: "editor.tab_size".into(),
            message: "must be 1–16".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("editor.tab_size"));
        assert!(msg.contains("must be 1–16"));
        assert!(msg.contains("validation error"));
    }

    #[test]
    fn io_error_display_contains_inner() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ConfigError::from(inner);
        let msg = format!("{err}");
        assert!(msg.contains("file missing"));
    }
}
