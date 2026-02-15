use crate::config::Config;
use crate::error::ConfigError;

/// Merge an overlay TOML fragment on top of a base [`Config`].
///
/// Values present in `overlay_toml` override those in `base`.
/// Missing keys in the overlay keep their `base` values.
/// Works by converting both sides to [`toml::Value`] tables,
/// deep-merging, then deserializing back to [`Config`].
pub fn merge_configs(base: &Config, overlay_toml: &str) -> Result<Config, ConfigError> {
    let base_str = toml::to_string(base).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let mut base_val: toml::Value =
        toml::from_str(&base_str).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let overlay_val: toml::Value =
        toml::from_str(overlay_toml).map_err(|e| ConfigError::Parse(e.to_string()))?;

    merge_values(&mut base_val, &overlay_val);

    let merged: Config = base_val
        .try_into()
        .map_err(|e: toml::de::Error| ConfigError::Parse(e.to_string()))?;

    Ok(merged)
}

/// Recursively merge `overlay` into `base`.
///
/// Tables are merged key-by-key; all other value types are
/// replaced outright.
fn merge_values(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, val) in overlay_table {
                if let Some(base_val) = base_table.get_mut(key) {
                    merge_values(base_val, val);
                } else {
                    base_table.insert(key.clone(), val.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_empty_overlay_returns_base() {
        let base = Config::default();
        let merged = merge_configs(&base, "").expect("merge empty");
        assert_eq!(merged, base);
    }

    #[test]
    fn merge_overrides_tab_size() {
        let base = Config::default();
        let overlay = "[editor]\ntab_size = 8\n";
        let merged = merge_configs(&base, overlay).expect("merge");
        assert_eq!(merged.editor.tab_size, 8);
        // Other editor values unchanged
        assert!(merged.editor.insert_spaces);
        assert!(merged.editor.auto_indent);
    }

    #[test]
    fn merge_overrides_nested_display_theme() {
        let base = Config::default();
        let overlay = "[display]\ntheme = \"solarized\"\n";
        let merged = merge_configs(&base, overlay).expect("merge");
        assert_eq!(merged.display.theme, "solarized");
        // Other display values unchanged
        assert!(merged.display.cursor_blink);
    }

    #[test]
    fn merge_adds_missing_field() {
        let base = Config::default();
        assert!(base.terminal_shell.is_none());
        let overlay = "terminal_shell = \"/bin/fish\"\n";
        let merged = merge_configs(&base, overlay).expect("merge");
        assert_eq!(merged.terminal_shell.as_deref(), Some("/bin/fish"),);
    }

    #[test]
    fn merge_invalid_overlay_returns_parse_error() {
        let base = Config::default();
        let result = merge_configs(&base, "{{invalid}}");
        assert!(result.is_err());
    }

    #[test]
    fn merge_preserves_unrelated_sections() {
        let base = Config::default();
        let overlay = "[editor]\ntab_size = 2\n";
        let merged = merge_configs(&base, overlay).expect("merge");
        // Display section unaffected
        assert_eq!(merged.display, base.display);
        assert_eq!(merged.keymap, base.keymap);
        assert_eq!(merged.log, base.log);
    }
}
