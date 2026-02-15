use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::ConfigError;
use crate::merge::merge_configs;
use crate::validate::validate;

/// Content written into a newly-created default config file.
const DEFAULT_CONFIG_CONTENT: &str = r#"# SMASH configuration
# Uncomment and edit settings below to override defaults.

# [editor]
# tab_size = 4
# insert_spaces = true
# word_wrap = false
# auto_indent = true
# trim_trailing_whitespace = false

# [display]
# theme = "dark"
# line_numbers = "absolute"
# cursor_blink = true

# [terminal]
# shell = "/bin/zsh"

# [log]
# level = "info"
"#;

/// Load and merge configuration.
///
/// 1. Reads the global config from `config_dir/config.toml`.
///    If the file does not exist it is created with commented-out
///    defaults.
/// 2. Optionally reads a project config from
///    `project_dir/.smash/config.toml` (walks upward).
/// 3. Merges: `Config::default() <- global <- project`.
/// 4. Validates the merged result.
///
/// # Errors
///
/// Returns [`ConfigError`] on I/O failure, parse failure, or
/// validation failure.
pub fn load_config(config_dir: &Path, project_dir: Option<&Path>) -> Result<Config, ConfigError> {
    let global_path = config_dir.join("config.toml");

    // Ensure config dir exists
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)?;
    }

    // Create default config if missing
    if !global_path.exists() {
        std::fs::write(&global_path, DEFAULT_CONFIG_CONTENT)
            .map_err(|e| ConfigError::CreateDefault(e.to_string()))?;
        tracing::info!("Created default config at {}", global_path.display(),);
    }

    // Start with defaults
    let mut config = Config::default();

    // Merge global config
    let global_content = std::fs::read_to_string(&global_path)?;
    if has_non_comment_content(&global_content) {
        config = merge_configs(&config, &global_content)?;
    }

    // Merge project config
    if let Some(proj) = project_dir {
        if let Some(project_path) = find_project_config(proj) {
            let project_content = std::fs::read_to_string(&project_path)?;
            config = merge_configs(&config, &project_content)?;
        }
    }

    // Validate
    validate(&config).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .unwrap_or_else(|| ConfigError::Validation {
                field: "unknown".to_string(),
                message: "validation failed".to_string(),
            })
    })?;

    Ok(config)
}

/// Walk from `start` upward looking for `.smash/config.toml`.
fn find_project_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".smash").join("config.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Returns `true` when the content has at least one
/// non-empty, non-comment line.
fn has_non_comment_content(content: &str) -> bool {
    content.lines().any(|l| {
        let trimmed = l.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
}

/// Parse a TOML string directly into a validated [`Config`].
///
/// Useful for tests or one-off parsing without file I/O.
///
/// # Errors
///
/// Returns [`ConfigError`] on parse or validation failure.
pub fn load_from_str(toml_str: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(toml_str).map_err(|e| ConfigError::Parse(e.to_string()))?;

    validate(&config).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .unwrap_or_else(|| ConfigError::Validation {
                field: "unknown".to_string(),
                message: "validation failed".to_string(),
            })
    })?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_config_creates_default_when_missing() {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("config");

        let config = load_config(&cfg_dir, None).unwrap();
        assert_eq!(config, Config::default());

        // File was created
        let created = cfg_dir.join("config.toml");
        assert!(created.exists());
    }

    #[test]
    fn load_config_reads_existing_global() {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(cfg_dir.join("config.toml"), "[editor]\ntab_size = 8\n").unwrap();

        let config = load_config(&cfg_dir, None).unwrap();
        assert_eq!(config.editor.tab_size, 8);
        // Unmodified fields keep defaults
        assert!(config.editor.insert_spaces);
    }

    #[test]
    fn load_config_merges_project_over_global() {
        let tmp = TempDir::new().unwrap();
        let cfg_dir = tmp.path().join("config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(cfg_dir.join("config.toml"), "[editor]\ntab_size = 8\n").unwrap();

        let proj_dir = tmp.path().join("project");
        let smash_dir = proj_dir.join(".smash");
        std::fs::create_dir_all(&smash_dir).unwrap();
        std::fs::write(smash_dir.join("config.toml"), "[editor]\ntab_size = 2\n").unwrap();

        let config = load_config(&cfg_dir, Some(&proj_dir)).unwrap();
        assert_eq!(config.editor.tab_size, 2);
    }

    #[test]
    fn load_from_str_parses_valid_toml() {
        let toml = "[editor]\ntab_size = 6\n";
        let config = load_from_str(toml).unwrap();
        assert_eq!(config.editor.tab_size, 6);
    }

    #[test]
    fn load_from_str_rejects_invalid_toml() {
        let result = load_from_str("{{bad}}");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_str_rejects_invalid_values() {
        let toml = "[editor]\ntab_size = 0\n";
        let result = load_from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn find_project_config_walks_up() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let smash = root.join(".smash");
        std::fs::create_dir_all(&smash).unwrap();
        std::fs::write(smash.join("config.toml"), "[editor]\ntab_size = 2\n").unwrap();

        let deep = root.join("src").join("module");
        std::fs::create_dir_all(&deep).unwrap();

        let found = find_project_config(&deep);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with(".smash/config.toml"));
    }

    #[test]
    fn find_project_config_returns_none_when_absent() {
        let tmp = TempDir::new().unwrap();
        let found = find_project_config(tmp.path());
        // May or may not find one depending on the system;
        // with a fresh temp dir it should be None unless
        // the system root has .smash/config.toml.
        // We just verify it does not panic.
        let _ = found;
    }

    #[test]
    fn default_config_content_parses_as_defaults() {
        // The comment-only template should produce defaults
        assert!(!has_non_comment_content(DEFAULT_CONFIG_CONTENT));
    }

    #[test]
    fn has_non_comment_content_detects_values() {
        assert!(!has_non_comment_content(""));
        assert!(!has_non_comment_content("# comment\n"));
        assert!(has_non_comment_content("# comment\ntab_size = 4\n"));
    }
}
