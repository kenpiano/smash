//! Breakpoint management for DAP sessions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A client-side breakpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct Breakpoint {
    /// Source file path.
    pub path: PathBuf,
    /// Line number (1-based).
    pub line: i64,
    /// Optional condition expression.
    pub condition: Option<String>,
    /// Optional hit condition expression.
    pub hit_condition: Option<String>,
    /// Optional log message (logpoint).
    pub log_message: Option<String>,
    /// Whether the adapter has verified this breakpoint.
    pub verified: bool,
    /// Adapter-assigned ID (set after adapter response).
    pub adapter_id: Option<i64>,
}

impl Breakpoint {
    /// Create a new unverified breakpoint at the given path and line.
    pub fn new(path: PathBuf, line: i64) -> Self {
        Self {
            path,
            line,
            condition: None,
            hit_condition: None,
            log_message: None,
            verified: false,
            adapter_id: None,
        }
    }

    /// Create a conditional breakpoint.
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Create a breakpoint with a hit condition.
    pub fn with_hit_condition(mut self, hit_condition: impl Into<String>) -> Self {
        self.hit_condition = Some(hit_condition.into());
        self
    }

    /// Create a logpoint.
    pub fn with_log_message(mut self, msg: impl Into<String>) -> Self {
        self.log_message = Some(msg.into());
        self
    }
}

/// Manages breakpoints across files for a debug session.
#[derive(Debug, Clone, Default)]
pub struct BreakpointManager {
    breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,
}

impl BreakpointManager {
    /// Create a new empty breakpoint manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breakpoint. Returns the index in the file's breakpoint list.
    pub fn add(&mut self, bp: Breakpoint) -> usize {
        let list = self.breakpoints.entry(bp.path.clone()).or_default();
        list.push(bp);
        list.len() - 1
    }

    /// Remove a breakpoint at the given path and line.
    ///
    /// Returns `true` if a breakpoint was removed.
    pub fn remove(&mut self, path: &Path, line: i64) -> bool {
        if let Some(list) = self.breakpoints.get_mut(path) {
            let before = list.len();
            list.retain(|bp| bp.line != line);
            if list.is_empty() {
                self.breakpoints.remove(path);
            }
            let after = self.breakpoints.get(path).map_or(0, |l| l.len());
            before != after
        } else {
            false
        }
    }

    /// Get all breakpoints for a file.
    pub fn get_for_file(&self, path: &Path) -> &[Breakpoint] {
        self.breakpoints.get(path).map_or(&[], |v| v.as_slice())
    }

    /// Mark a breakpoint as verified by the adapter.
    pub fn mark_verified(&mut self, path: &Path, line: i64, adapter_id: Option<i64>) {
        if let Some(list) = self.breakpoints.get_mut(path) {
            for bp in list.iter_mut() {
                if bp.line == line {
                    bp.verified = true;
                    bp.adapter_id = adapter_id;
                }
            }
        }
    }

    /// Remove all breakpoints for a specific file.
    pub fn clear_file(&mut self, path: &Path) {
        self.breakpoints.remove(path);
    }

    /// Return an iterator over all breakpoints across all files.
    pub fn all(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints.values().flat_map(|v| v.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/src/{name}"))
    }

    #[test]
    fn breakpoint_set_and_verify() {
        let mut mgr = BreakpointManager::new();
        let path = test_path("main.rs");

        let bp = Breakpoint::new(path.clone(), 10);
        mgr.add(bp);

        let bps = mgr.get_for_file(&path);
        assert_eq!(bps.len(), 1);
        assert!(!bps[0].verified);

        mgr.mark_verified(&path, 10, Some(1));

        let bps = mgr.get_for_file(&path);
        assert!(bps[0].verified);
        assert_eq!(bps[0].adapter_id, Some(1));
    }

    #[test]
    fn breakpoint_conditional() {
        let mut mgr = BreakpointManager::new();
        let path = test_path("lib.rs");

        let bp = Breakpoint::new(path.clone(), 42).with_condition("x > 10");
        mgr.add(bp);

        let bps = mgr.get_for_file(&path);
        assert_eq!(bps.len(), 1);
        assert_eq!(bps[0].condition.as_deref(), Some("x > 10"));
    }

    #[test]
    fn breakpoint_remove() {
        let mut mgr = BreakpointManager::new();
        let path = test_path("main.rs");

        mgr.add(Breakpoint::new(path.clone(), 10));
        mgr.add(Breakpoint::new(path.clone(), 20));

        assert!(mgr.remove(&path, 10));
        assert_eq!(mgr.get_for_file(&path).len(), 1);
        assert_eq!(mgr.get_for_file(&path)[0].line, 20);

        // Removing a non-existent line returns false.
        assert!(!mgr.remove(&path, 999));
    }

    #[test]
    fn breakpoint_unverified() {
        let bp = Breakpoint::new(PathBuf::from("/test.rs"), 5);
        assert!(!bp.verified);
        assert_eq!(bp.adapter_id, None);
    }

    #[test]
    fn breakpoint_multiple_files() {
        let mut mgr = BreakpointManager::new();
        let path_a = test_path("a.rs");
        let path_b = test_path("b.rs");

        mgr.add(Breakpoint::new(path_a.clone(), 1));
        mgr.add(Breakpoint::new(path_a.clone(), 2));
        mgr.add(Breakpoint::new(path_b.clone(), 10));

        assert_eq!(mgr.get_for_file(&path_a).len(), 2);
        assert_eq!(mgr.get_for_file(&path_b).len(), 1);
        assert_eq!(mgr.all().count(), 3);

        mgr.clear_file(&path_a);
        assert_eq!(mgr.get_for_file(&path_a).len(), 0);
        assert_eq!(mgr.all().count(), 1);
    }

    #[test]
    fn breakpoint_with_hit_condition() {
        let bp = Breakpoint::new(PathBuf::from("/test.rs"), 3).with_hit_condition("== 5");
        assert_eq!(bp.hit_condition.as_deref(), Some("== 5"));
    }

    #[test]
    fn breakpoint_with_log_message() {
        let bp = Breakpoint::new(PathBuf::from("/test.rs"), 3).with_log_message("value is {x}");
        assert_eq!(bp.log_message.as_deref(), Some("value is {x}"));
    }

    #[test]
    fn breakpoint_remove_last_clears_file_entry() {
        let mut mgr = BreakpointManager::new();
        let path = test_path("single.rs");
        mgr.add(Breakpoint::new(path.clone(), 1));
        assert!(mgr.remove(&path, 1));
        // The file entry should be cleaned up.
        assert_eq!(mgr.get_for_file(&path).len(), 0);
        assert_eq!(mgr.all().count(), 0);
    }
}
