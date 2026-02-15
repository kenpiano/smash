//! Logging subsystem helpers (REQ-NFR-020, REQ-NFR-021).
//!
//! Provides log-file rotation, default path resolution, and level conversion.
//! The actual `tracing-subscriber` setup lives in the binary crate (`src/main.rs`)
//! because `tracing-subscriber` is only a binary dependency.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Maximum size of a single log file before rotation (10 MB).
pub const DEFAULT_MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum number of rotated log files to retain.
pub const DEFAULT_MAX_LOG_FILES: u32 = 5;

/// Return the platform-specific default log file path.
///
/// * macOS: `$HOME/Library/Logs/smash/smash.log`
/// * Linux: `$HOME/.local/share/smash/smash.log`
/// * Windows: `%APPDATA%/smash/logs/smash.log`
/// * Fallback: `/tmp/smash/smash.log`
pub fn default_log_file_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join("Library/Logs/smash/smash.log");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local/share/smash/smash.log");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("smash\\logs\\smash.log");
        }
    }
    PathBuf::from("/tmp/smash/smash.log")
}

/// Ensure the parent directory of a log file exists, creating it if necessary.
pub fn ensure_log_dir(log_path: &Path) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

/// Rotate log files when the current file exceeds `max_size` bytes.
///
/// Rotation scheme:
/// ```text
///   smash.log   → smash.log.1
///   smash.log.1 → smash.log.2
///   …
///   smash.log.<max_files> is deleted
/// ```
///
/// Does nothing when the file does not exist or is smaller than `max_size`.
pub fn rotate_log_files(log_path: &Path, max_size: u64, max_files: u32) -> io::Result<()> {
    if !log_path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(log_path)?;
    if metadata.len() < max_size {
        return Ok(());
    }

    // Delete the oldest rotated file.
    let oldest = rotated_path(log_path, max_files);
    if oldest.exists() {
        fs::remove_file(&oldest)?;
    }

    // Shift existing rotated files upward.
    for i in (1..max_files).rev() {
        let from = rotated_path(log_path, i);
        let to = rotated_path(log_path, i + 1);
        if from.exists() {
            fs::rename(&from, &to)?;
        }
    }

    // Rotate the current log to .1
    let first_rotated = rotated_path(log_path, 1);
    fs::rename(log_path, &first_rotated)?;

    Ok(())
}

/// Convert a log level name (case-insensitive) to a `tracing`-compatible
/// filter string.  Returns `"info"` for unrecognised values.
pub fn log_level_to_filter(level: &str) -> &'static str {
    match level.to_ascii_lowercase().as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    }
}

// ── internal helpers ────────────────────────────────────────────────────────

fn rotated_path(base: &Path, index: u32) -> PathBuf {
    let name = base.file_name().unwrap_or_default().to_string_lossy();
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.{}", name, index))
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_file_path_contains_smash() {
        let path = default_log_file_path();
        assert!(
            path.to_string_lossy().contains("smash"),
            "expected 'smash' in path, got: {:?}",
            path
        );
    }

    #[test]
    fn default_log_file_path_ends_with_log_extension() {
        let path = default_log_file_path();
        assert!(
            path.extension().is_some_and(|e| e == "log"),
            "expected .log extension, got: {:?}",
            path
        );
    }

    #[test]
    fn rotated_path_format_is_correct() {
        let base = Path::new("/tmp/smash.log");
        assert_eq!(rotated_path(base, 1), PathBuf::from("/tmp/smash.log.1"));
        assert_eq!(rotated_path(base, 3), PathBuf::from("/tmp/smash.log.3"));
    }

    #[test]
    fn rotate_no_op_when_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("smash.log");
        let result = rotate_log_files(&log, DEFAULT_MAX_LOG_SIZE, DEFAULT_MAX_LOG_FILES);
        assert!(result.is_ok());
    }

    #[test]
    fn rotate_no_op_when_file_under_max_size() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("smash.log");
        fs::write(&log, "small content").unwrap();
        rotate_log_files(&log, DEFAULT_MAX_LOG_SIZE, DEFAULT_MAX_LOG_FILES).unwrap();
        assert!(log.exists(), "file should remain when under max size");
    }

    #[test]
    fn rotate_shifts_file_to_dot_one() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("smash.log");
        fs::write(&log, "x".repeat(100)).unwrap();
        rotate_log_files(&log, 50, 3).unwrap();
        assert!(!log.exists(), "original should be gone after rotation");
        assert!(
            dir.path().join("smash.log.1").exists(),
            "rotated .1 should exist"
        );
    }

    #[test]
    fn rotate_cascades_existing_rotated_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("smash.log");

        // Pre-existing rotated files.
        fs::write(dir.path().join("smash.log.1"), "old1").unwrap();
        fs::write(dir.path().join("smash.log.2"), "old2").unwrap();

        // Current log exceeds threshold.
        fs::write(&log, "x".repeat(200)).unwrap();
        rotate_log_files(&log, 50, 3).unwrap();

        assert!(!log.exists());
        assert!(dir.path().join("smash.log.1").exists());
        assert!(dir.path().join("smash.log.2").exists());
        assert!(dir.path().join("smash.log.3").exists());

        // Verify the cascade: current → .1, old .1 → .2, old .2 → .3
        assert_eq!(
            fs::read_to_string(dir.path().join("smash.log.2")).unwrap(),
            "old1"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("smash.log.3")).unwrap(),
            "old2"
        );
    }

    #[test]
    fn rotate_deletes_oldest_beyond_max_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("smash.log");

        // Pre-existing rotated files reaching the max (max_files=2).
        fs::write(dir.path().join("smash.log.1"), "old1").unwrap();
        fs::write(dir.path().join("smash.log.2"), "old2").unwrap();

        fs::write(&log, "x".repeat(200)).unwrap();
        rotate_log_files(&log, 50, 2).unwrap();

        assert!(!log.exists());
        assert!(dir.path().join("smash.log.1").exists());
        assert!(dir.path().join("smash.log.2").exists());
        // old .2 should have been deleted, and old .1 shifted to .2
        assert_eq!(
            fs::read_to_string(dir.path().join("smash.log.2")).unwrap(),
            "old1"
        );
    }

    #[test]
    fn ensure_log_dir_creates_nested_parents() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("a").join("b").join("c").join("smash.log");
        ensure_log_dir(&log).unwrap();
        assert!(dir.path().join("a").join("b").join("c").exists());
    }

    #[test]
    fn ensure_log_dir_is_idempotent() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = dir.path().join("sub").join("smash.log");
        ensure_log_dir(&log).unwrap();
        ensure_log_dir(&log).unwrap(); // second call must not fail
        assert!(dir.path().join("sub").exists());
    }

    #[test]
    fn log_level_to_filter_known_levels() {
        assert_eq!(log_level_to_filter("trace"), "trace");
        assert_eq!(log_level_to_filter("debug"), "debug");
        assert_eq!(log_level_to_filter("info"), "info");
        assert_eq!(log_level_to_filter("warn"), "warn");
        assert_eq!(log_level_to_filter("error"), "error");
    }

    #[test]
    fn log_level_to_filter_case_insensitive() {
        assert_eq!(log_level_to_filter("TRACE"), "trace");
        assert_eq!(log_level_to_filter("Debug"), "debug");
        assert_eq!(log_level_to_filter("INFO"), "info");
    }

    #[test]
    fn log_level_to_filter_defaults_unknown_to_info() {
        assert_eq!(log_level_to_filter("invalid"), "info");
        assert_eq!(log_level_to_filter(""), "info");
        assert_eq!(log_level_to_filter("verbose"), "info");
    }
}
