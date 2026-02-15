use std::path::PathBuf;

use crate::error::PlatformError;

/// Trait providing standard directory paths for the application.
pub trait PlatformPaths: Send + Sync {
    /// Returns the configuration directory (`~/.config/smash`).
    fn config_dir(&self) -> PathBuf;
    /// Returns the data directory (`~/.local/share/smash`).
    fn data_dir(&self) -> PathBuf;
    /// Returns the cache directory (`~/.cache/smash`).
    fn cache_dir(&self) -> PathBuf;
    /// Returns the log directory (`<data_dir>/logs`).
    fn log_dir(&self) -> PathBuf;
    /// Returns the default shell executable path.
    fn default_shell(&self) -> PathBuf;
    /// Returns the user's home directory.
    fn home_dir(&self) -> PathBuf;
}

/// Default implementation of [`PlatformPaths`] using the `dirs` crate and
/// environment variables.
pub struct DefaultPaths {
    home: PathBuf,
}

impl DefaultPaths {
    /// Creates a new `DefaultPaths` instance, resolving the home directory.
    ///
    /// # Errors
    ///
    /// Returns `PlatformError::Path` if the home directory cannot be
    /// determined.
    pub fn new() -> Result<Self, PlatformError> {
        let home = dirs::home_dir()
            .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
            .ok_or_else(|| PlatformError::Path("could not determine home directory".into()))?;
        Ok(Self { home })
    }
}

impl PlatformPaths for DefaultPaths {
    fn config_dir(&self) -> PathBuf {
        self.home.join(".config").join("smash")
    }

    fn data_dir(&self) -> PathBuf {
        self.home.join(".local").join("share").join("smash")
    }

    fn cache_dir(&self) -> PathBuf {
        self.home.join(".cache").join("smash")
    }

    fn log_dir(&self) -> PathBuf {
        self.data_dir().join("logs")
    }

    fn default_shell(&self) -> PathBuf {
        std::env::var("SHELL")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/bin/sh"))
    }

    fn home_dir(&self) -> PathBuf {
        self.home.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paths() -> DefaultPaths {
        DefaultPaths::new().expect("should resolve home directory")
    }

    #[test]
    fn config_dir_ends_with_config_smash() {
        let paths = make_paths();
        let config = paths.config_dir();
        assert!(
            config.ends_with(".config/smash"),
            "config_dir should end with .config/smash, got: {:?}",
            config
        );
    }

    #[test]
    fn config_dir_contains_smash() {
        let paths = make_paths();
        let config = paths.config_dir();
        let s = config.to_string_lossy();
        assert!(s.contains("smash"), "config_dir should contain 'smash'");
    }

    #[test]
    fn data_dir_contains_smash() {
        let paths = make_paths();
        let data = paths.data_dir();
        let s = data.to_string_lossy();
        assert!(s.contains("smash"), "data_dir should contain 'smash'");
    }

    #[test]
    fn cache_dir_contains_smash() {
        let paths = make_paths();
        let cache = paths.cache_dir();
        let s = cache.to_string_lossy();
        assert!(s.contains("smash"), "cache_dir should contain 'smash'");
    }

    #[test]
    fn log_dir_contains_smash() {
        let paths = make_paths();
        let log = paths.log_dir();
        let s = log.to_string_lossy();
        assert!(s.contains("smash"), "log_dir should contain 'smash'");
    }

    #[test]
    fn log_dir_is_under_data_dir() {
        let paths = make_paths();
        let log = paths.log_dir();
        let data = paths.data_dir();
        assert!(log.starts_with(&data), "log_dir should be under data_dir");
    }

    #[test]
    fn default_shell_is_non_empty() {
        let paths = make_paths();
        let shell = paths.default_shell();
        assert!(
            !shell.as_os_str().is_empty(),
            "default_shell should be non-empty"
        );
    }

    #[test]
    fn home_dir_is_non_empty() {
        let paths = make_paths();
        let home = paths.home_dir();
        assert!(!home.as_os_str().is_empty(), "home_dir should be non-empty");
    }

    #[test]
    fn default_paths_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DefaultPaths>();
    }
}
