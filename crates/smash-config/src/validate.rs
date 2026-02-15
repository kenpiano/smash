use crate::config::Config;
use crate::error::ConfigError;

/// Validate a [`Config`], returning all detected violations.
///
/// Returns `Ok(())` when the config is valid, or `Err` with a
/// vector of every validation error found.
pub fn validate(config: &Config) -> Result<(), Vec<ConfigError>> {
    let mut errors = Vec::new();

    // tab_size: 1–16
    if config.editor.tab_size == 0 || config.editor.tab_size > 16 {
        errors.push(ConfigError::Validation {
            field: "editor.tab_size".to_string(),
            message: format!("must be 1\u{2013}16, got {}", config.editor.tab_size,),
        });
    }

    // theme: non-empty
    if config.display.theme.is_empty() {
        errors.push(ConfigError::Validation {
            field: "display.theme".to_string(),
            message: "must not be empty".to_string(),
        });
    }

    // auto_save_interval_secs: 0 (disabled) or >= 5
    if config.auto_save_interval_secs > 0 && config.auto_save_interval_secs < 5 {
        errors.push(ConfigError::Validation {
            field: "auto_save_interval_secs".to_string(),
            message: format!(
                "must be 0 (disabled) or \u{2265} 5, got {}",
                config.auto_save_interval_secs,
            ),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_default_config_passes() {
        let cfg = Config::default();
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn invalid_tab_size_zero() {
        let mut cfg = Config::default();
        cfg.editor.tab_size = 0;
        let errs = validate(&cfg).unwrap_err();
        assert_eq!(errs.len(), 1);
        let msg = format!("{}", errs[0]);
        assert!(msg.contains("editor.tab_size"));
    }

    #[test]
    fn invalid_tab_size_seventeen() {
        let mut cfg = Config::default();
        cfg.editor.tab_size = 17;
        let errs = validate(&cfg).unwrap_err();
        assert_eq!(errs.len(), 1);
        let msg = format!("{}", errs[0]);
        assert!(msg.contains("editor.tab_size"));
    }

    #[test]
    fn empty_theme_rejected() {
        let mut cfg = Config::default();
        cfg.display.theme = String::new();
        let errs = validate(&cfg).unwrap_err();
        assert_eq!(errs.len(), 1);
        let msg = format!("{}", errs[0]);
        assert!(msg.contains("display.theme"));
    }

    #[test]
    fn auto_save_three_rejected() {
        let cfg = Config {
            auto_save_interval_secs: 3,
            ..Config::default()
        };
        let errs = validate(&cfg).unwrap_err();
        assert_eq!(errs.len(), 1);
        let msg = format!("{}", errs[0]);
        assert!(msg.contains("auto_save_interval_secs"));
    }

    #[test]
    fn auto_save_zero_allowed() {
        let cfg = Config {
            auto_save_interval_secs: 0,
            ..Config::default()
        };
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn auto_save_five_allowed() {
        let cfg = Config {
            auto_save_interval_secs: 5,
            ..Config::default()
        };
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn multiple_errors_returned() {
        let mut cfg = Config::default();
        cfg.editor.tab_size = 0;
        cfg.display.theme = String::new();
        cfg.auto_save_interval_secs = 2;
        let errs = validate(&cfg).unwrap_err();
        assert_eq!(errs.len(), 3);
    }
}
