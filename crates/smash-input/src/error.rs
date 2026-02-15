use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("invalid key sequence: {0}")]
    InvalidKeySequence(String),
    #[error("keymap parse error: {0}")]
    KeymapParse(String),
    #[error("duplicate binding for {key} in layer {layer}")]
    DuplicateBinding { key: String, layer: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_display_shows_name() {
        let err = InputError::UnknownCommand("foo".into());
        assert_eq!(err.to_string(), "unknown command: foo");
    }

    #[test]
    fn invalid_key_sequence_display_shows_sequence() {
        let err = InputError::InvalidKeySequence("X-Y-Z".into());
        assert_eq!(err.to_string(), "invalid key sequence: X-Y-Z");
    }

    #[test]
    fn keymap_parse_display_shows_detail() {
        let err = InputError::KeymapParse("bad toml".into());
        assert_eq!(err.to_string(), "keymap parse error: bad toml");
    }

    #[test]
    fn duplicate_binding_display_shows_key_and_layer() {
        let err = InputError::DuplicateBinding {
            key: "Ctrl-S".into(),
            layer: "default".into(),
        };
        assert_eq!(
            err.to_string(),
            "duplicate binding for Ctrl-S in layer default"
        );
    }
}
