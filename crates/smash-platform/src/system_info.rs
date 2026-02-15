/// Represents the operating system kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsKind {
    Linux,
    MacOs,
    Windows,
    Unknown,
}

/// Represents the CPU architecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Unknown,
}

/// Runtime system information detected at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    os: OsKind,
    arch: Arch,
}

impl SystemInfo {
    /// Detects the current OS and architecture at compile time.
    pub fn detect() -> Self {
        let os = if cfg!(target_os = "macos") {
            OsKind::MacOs
        } else if cfg!(target_os = "linux") {
            OsKind::Linux
        } else if cfg!(target_os = "windows") {
            OsKind::Windows
        } else {
            OsKind::Unknown
        };

        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else {
            Arch::Unknown
        };

        Self { os, arch }
    }

    /// Returns the detected OS kind.
    pub fn os(&self) -> &OsKind {
        &self.os
    }

    /// Returns the detected CPU architecture.
    pub fn arch(&self) -> &Arch {
        &self.arch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_os_kind() {
        let info = SystemInfo::detect();
        // On any supported CI / dev machine, os should not be Unknown.
        assert!(
            matches!(info.os(), OsKind::MacOs | OsKind::Linux | OsKind::Windows),
            "expected a known OS, got: {:?}",
            info.os()
        );
    }

    #[test]
    fn detect_returns_valid_arch() {
        let info = SystemInfo::detect();
        assert!(
            matches!(info.arch(), Arch::X86_64 | Arch::Aarch64),
            "expected a known arch, got: {:?}",
            info.arch()
        );
    }

    #[test]
    fn detect_is_consistent() {
        let a = SystemInfo::detect();
        let b = SystemInfo::detect();
        assert_eq!(a, b, "consecutive calls should return the same info");
    }

    #[test]
    fn system_info_is_clone() {
        let info = SystemInfo::detect();
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn system_info_debug_format() {
        let info = SystemInfo::detect();
        let debug = format!("{:?}", info);
        assert!(!debug.is_empty(), "Debug format should not be empty");
    }

    #[test]
    fn system_info_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SystemInfo>();
    }
}
