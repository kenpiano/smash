use std::sync::Mutex;

use crate::error::PlatformError;

/// Trait for clipboard operations.
pub trait Clipboard: Send + Sync {
    /// Retrieves the current clipboard content.
    fn get(&self) -> Result<String, PlatformError>;
    /// Sets the clipboard content.
    fn set(&self, content: &str) -> Result<(), PlatformError>;
}

/// System clipboard that shells out to OS-specific commands.
///
/// On macOS, uses `pbcopy` / `pbpaste`.
/// On Linux, uses `xclip` (with `-selection clipboard`).
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn get(&self) -> Result<String, PlatformError> {
        let cmd = if cfg!(target_os = "macos") {
            ("pbpaste", Vec::new())
        } else if cfg!(target_os = "linux") {
            ("xclip", vec!["-selection", "clipboard", "-o"])
        } else {
            return Err(PlatformError::Unsupported {
                os: std::env::consts::OS.into(),
                detail: "clipboard get not supported".into(),
            });
        };

        let output = std::process::Command::new(cmd.0)
            .args(&cmd.1)
            .output()
            .map_err(|e| PlatformError::Clipboard(format!("failed to run {}: {}", cmd.0, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::Clipboard(format!(
                "{} failed: {}",
                cmd.0, stderr
            )));
        }

        String::from_utf8(output.stdout).map_err(|e| {
            PlatformError::Clipboard(format!("clipboard content is not valid UTF-8: {}", e))
        })
    }

    fn set(&self, content: &str) -> Result<(), PlatformError> {
        let cmd = if cfg!(target_os = "macos") {
            ("pbcopy", Vec::new())
        } else if cfg!(target_os = "linux") {
            ("xclip", vec!["-selection", "clipboard"])
        } else {
            return Err(PlatformError::Unsupported {
                os: std::env::consts::OS.into(),
                detail: "clipboard set not supported".into(),
            });
        };

        use std::io::Write;
        let mut child = std::process::Command::new(cmd.0)
            .args(&cmd.1)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| PlatformError::Clipboard(format!("failed to spawn {}: {}", cmd.0, e)))?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(content.as_bytes()).map_err(|e| {
                PlatformError::Clipboard(format!("failed to write to {}: {}", cmd.0, e))
            })?;
        }

        let status = child
            .wait()
            .map_err(|e| PlatformError::Clipboard(format!("failed to wait on {}: {}", cmd.0, e)))?;

        if !status.success() {
            return Err(PlatformError::Clipboard(format!(
                "{} exited with status: {}",
                cmd.0, status
            )));
        }

        Ok(())
    }
}

/// In-memory clipboard for testing purposes.
pub struct InMemoryClipboard {
    content: Mutex<String>,
}

impl InMemoryClipboard {
    /// Creates a new empty in-memory clipboard.
    pub fn new() -> Self {
        Self {
            content: Mutex::new(String::new()),
        }
    }
}

impl Default for InMemoryClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard for InMemoryClipboard {
    fn get(&self) -> Result<String, PlatformError> {
        let guard = self
            .content
            .lock()
            .map_err(|e| PlatformError::Clipboard(format!("mutex poisoned: {}", e)))?;
        Ok(guard.clone())
    }

    fn set(&self, content: &str) -> Result<(), PlatformError> {
        let mut guard = self
            .content
            .lock()
            .map_err(|e| PlatformError::Clipboard(format!("mutex poisoned: {}", e)))?;
        *guard = content.to_string();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_clipboard_set_and_get() {
        let cb = InMemoryClipboard::new();
        cb.set("hello world").expect("set should succeed");
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn in_memory_clipboard_set_empty_string() {
        let cb = InMemoryClipboard::new();
        cb.set("something").expect("set should succeed");
        cb.set("").expect("set empty should succeed");
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, "");
    }

    #[test]
    fn in_memory_clipboard_overwrite() {
        let cb = InMemoryClipboard::new();
        cb.set("first").expect("set should succeed");
        cb.set("second").expect("set should succeed");
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, "second");
    }

    #[test]
    fn in_memory_clipboard_default_is_empty() {
        let cb = InMemoryClipboard::default();
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, "");
    }

    #[test]
    fn in_memory_clipboard_unicode_content() {
        let cb = InMemoryClipboard::new();
        let text = "こんにちは 🌍";
        cb.set(text).expect("set should succeed");
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, text);
    }

    #[test]
    fn in_memory_clipboard_multiline_content() {
        let cb = InMemoryClipboard::new();
        let text = "line1\nline2\nline3";
        cb.set(text).expect("set should succeed");
        let result = cb.get().expect("get should succeed");
        assert_eq!(result, text);
    }

    #[test]
    fn in_memory_clipboard_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryClipboard>();
    }

    #[test]
    fn system_clipboard_round_trip() {
        // This test only runs on macOS/Linux where pbcopy/xclip exists
        let cb = SystemClipboard;
        let unique = format!("smash_test_{}", std::process::id());
        if cb.set(&unique).is_ok() {
            let got = cb.get().expect("get should succeed");
            assert_eq!(got, unique);
        }
        // If set fails (no clipboard tool), test is skipped
    }
}
