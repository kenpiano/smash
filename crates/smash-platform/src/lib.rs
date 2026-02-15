pub mod clipboard;
pub mod error;
pub mod paths;
pub mod system_info;

pub use clipboard::{Clipboard, InMemoryClipboard, SystemClipboard};
pub use error::PlatformError;
pub use paths::{DefaultPaths, PlatformPaths};
pub use system_info::{Arch, OsKind, SystemInfo};

/// Container holding all platform services.
pub struct Platform {
    clipboard: Box<dyn Clipboard>,
    paths: Box<dyn PlatformPaths>,
    system_info: SystemInfo,
}

impl Platform {
    /// Creates a new `Platform` with the given clipboard and paths
    /// implementations.
    pub fn new(clipboard: Box<dyn Clipboard>, paths: Box<dyn PlatformPaths>) -> Self {
        Self {
            clipboard,
            paths,
            system_info: SystemInfo::detect(),
        }
    }

    /// Creates a `Platform` with the default OS-level implementations.
    ///
    /// # Errors
    ///
    /// Returns `PlatformError` if the home directory cannot be resolved.
    pub fn default_platform() -> Result<Self, PlatformError> {
        Ok(Self::new(
            Box::new(SystemClipboard),
            Box::new(DefaultPaths::new()?),
        ))
    }

    /// Returns a reference to the clipboard service.
    pub fn clipboard(&self) -> &dyn Clipboard {
        self.clipboard.as_ref()
    }

    /// Returns a reference to the platform paths service.
    pub fn paths(&self) -> &dyn PlatformPaths {
        self.paths.as_ref()
    }

    /// Returns a reference to the detected system information.
    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_created_with_in_memory_clipboard_and_default_paths() {
        let paths = DefaultPaths::new().expect("should resolve home");
        let platform = Platform::new(Box::new(InMemoryClipboard::new()), Box::new(paths));

        // Clipboard should work
        platform
            .clipboard()
            .set("test")
            .expect("set should succeed");
        let result = platform.clipboard().get().expect("get should succeed");
        assert_eq!(result, "test");
    }

    #[test]
    fn platform_paths_accessor_returns_valid_config_dir() {
        let paths = DefaultPaths::new().expect("should resolve home");
        let platform = Platform::new(Box::new(InMemoryClipboard::new()), Box::new(paths));

        let config = platform.paths().config_dir();
        assert!(config.to_string_lossy().contains("smash"));
    }

    #[test]
    fn platform_system_info_accessor_returns_detected_info() {
        let paths = DefaultPaths::new().expect("should resolve home");
        let platform = Platform::new(Box::new(InMemoryClipboard::new()), Box::new(paths));

        let info = platform.system_info();
        assert!(matches!(
            info.os(),
            OsKind::MacOs | OsKind::Linux | OsKind::Windows
        ));
    }

    #[test]
    fn platform_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Platform>();
    }
}
