//! Diagnostics collector for LSP.
//!
//! Stores diagnostics per-URI received from language servers and
//! notifies the editor core via a channel when diagnostics change.
use std::collections::HashMap;

use crate::types::Diagnostic;

/// Stores diagnostics received from language servers, keyed by URI.
pub struct DiagnosticStore {
    /// Diagnostics per document URI.
    store: HashMap<String, Vec<Diagnostic>>,
    /// Callback invoked when diagnostics for a URI change.
    #[allow(clippy::type_complexity)]
    on_update: Option<Box<dyn Fn(&str, &[Diagnostic]) + Send + Sync>>,
}

impl Default for DiagnosticStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DiagnosticStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiagnosticStore")
            .field("store", &self.store)
            .field("has_on_update", &self.on_update.is_some())
            .finish()
    }
}

impl DiagnosticStore {
    /// Create a new empty diagnostic store.
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            on_update: None,
        }
    }

    /// Set a callback that fires whenever diagnostics for a URI change.
    pub fn set_on_update<F>(&mut self, callback: F)
    where
        F: Fn(&str, &[Diagnostic]) + Send + Sync + 'static,
    {
        self.on_update = Some(Box::new(callback));
    }

    /// Update diagnostics for a given URI.
    ///
    /// If the diagnostics list is empty, the entry is removed (cleared).
    /// Fires the on_update callback if set.
    pub fn publish(&mut self, uri: String, diagnostics: Vec<Diagnostic>) {
        if diagnostics.is_empty() {
            self.store.remove(&uri);
        } else {
            self.store.insert(uri.clone(), diagnostics);
        }
        if let Some(callback) = &self.on_update {
            let diags = self.get(&uri);
            callback(&uri, diags);
        }
    }

    /// Get diagnostics for a specific URI.
    pub fn get(&self, uri: &str) -> &[Diagnostic] {
        self.store.get(uri).map_or(&[], |v| v.as_slice())
    }

    /// Get all URIs that have diagnostics.
    pub fn uris(&self) -> Vec<&str> {
        self.store.keys().map(|s| s.as_str()).collect()
    }

    /// Get total number of diagnostics across all files.
    pub fn total_count(&self) -> usize {
        self.store.values().map(|v| v.len()).sum()
    }

    /// Get the number of files with diagnostics.
    pub fn file_count(&self) -> usize {
        self.store.len()
    }

    /// Check if a specific URI has any diagnostics.
    pub fn has_diagnostics(&self, uri: &str) -> bool {
        self.store.contains_key(uri)
    }

    /// Clear all diagnostics.
    pub fn clear_all(&mut self) {
        self.store.clear();
    }

    /// Clear diagnostics for a specific URI.
    pub fn clear(&mut self, uri: &str) {
        self.store.remove(uri);
    }

    /// Get an iterator over all (uri, diagnostics) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[Diagnostic])> {
        self.store.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// Get diagnostics for a URI at a specific line.
    pub fn diagnostics_at_line(&self, uri: &str, line: u32) -> Vec<&Diagnostic> {
        self.get(uri)
            .iter()
            .filter(|d| d.range.start.line <= line && d.range.end.line >= line)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiagnosticSeverity, LspPosition, LspRange};

    fn make_diagnostic(line: u32, message: &str, severity: DiagnosticSeverity) -> Diagnostic {
        Diagnostic {
            range: LspRange::new(LspPosition::new(line, 0), LspPosition::new(line, 10)),
            severity: Some(severity),
            message: message.to_string(),
            source: Some("test".to_string()),
            code: None,
        }
    }

    #[test]
    fn store_new_empty() {
        let store = DiagnosticStore::new();
        assert_eq!(store.total_count(), 0);
        assert_eq!(store.file_count(), 0);
    }

    #[test]
    fn store_default_empty() {
        let store = DiagnosticStore::default();
        assert_eq!(store.total_count(), 0);
    }

    #[test]
    fn store_publish_and_get() {
        let mut store = DiagnosticStore::new();
        let diags = vec![make_diagnostic(0, "error here", DiagnosticSeverity::Error)];
        store.publish("file:///test.rs".to_string(), diags);

        let retrieved = store.get("file:///test.rs");
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].message, "error here");
    }

    #[test]
    fn store_per_uri() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///a.rs".to_string(),
            vec![make_diagnostic(0, "err a", DiagnosticSeverity::Error)],
        );
        store.publish(
            "file:///b.rs".to_string(),
            vec![make_diagnostic(5, "err b", DiagnosticSeverity::Warning)],
        );

        assert_eq!(store.file_count(), 2);
        assert_eq!(store.total_count(), 2);
        assert_eq!(store.get("file:///a.rs")[0].message, "err a");
        assert_eq!(store.get("file:///b.rs")[0].message, "err b");
    }

    #[test]
    fn store_update_replaces() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///test.rs".to_string(),
            vec![make_diagnostic(0, "old error", DiagnosticSeverity::Error)],
        );
        store.publish(
            "file:///test.rs".to_string(),
            vec![make_diagnostic(1, "new error", DiagnosticSeverity::Error)],
        );

        let retrieved = store.get("file:///test.rs");
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].message, "new error");
    }

    #[test]
    fn store_clear_on_empty_publish() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///test.rs".to_string(),
            vec![make_diagnostic(0, "error", DiagnosticSeverity::Error)],
        );
        assert!(store.has_diagnostics("file:///test.rs"));

        // Publish empty list should clear
        store.publish("file:///test.rs".to_string(), vec![]);
        assert!(!store.has_diagnostics("file:///test.rs"));
        assert_eq!(store.file_count(), 0);
    }

    #[test]
    fn store_get_nonexistent_uri() {
        let store = DiagnosticStore::new();
        assert!(store.get("file:///nonexistent.rs").is_empty());
    }

    #[test]
    fn store_uris() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///a.rs".to_string(),
            vec![make_diagnostic(0, "x", DiagnosticSeverity::Error)],
        );
        store.publish(
            "file:///b.rs".to_string(),
            vec![make_diagnostic(0, "y", DiagnosticSeverity::Error)],
        );

        let uris = store.uris();
        assert_eq!(uris.len(), 2);
        assert!(uris.contains(&"file:///a.rs"));
        assert!(uris.contains(&"file:///b.rs"));
    }

    #[test]
    fn store_clear_specific_uri() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///a.rs".to_string(),
            vec![make_diagnostic(0, "x", DiagnosticSeverity::Error)],
        );
        store.publish(
            "file:///b.rs".to_string(),
            vec![make_diagnostic(0, "y", DiagnosticSeverity::Error)],
        );

        store.clear("file:///a.rs");
        assert!(!store.has_diagnostics("file:///a.rs"));
        assert!(store.has_diagnostics("file:///b.rs"));
    }

    #[test]
    fn store_clear_all() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///a.rs".to_string(),
            vec![make_diagnostic(0, "x", DiagnosticSeverity::Error)],
        );
        store.publish(
            "file:///b.rs".to_string(),
            vec![make_diagnostic(0, "y", DiagnosticSeverity::Error)],
        );

        store.clear_all();
        assert_eq!(store.total_count(), 0);
        assert_eq!(store.file_count(), 0);
    }

    #[test]
    fn store_iter() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///a.rs".to_string(),
            vec![make_diagnostic(0, "x", DiagnosticSeverity::Error)],
        );

        let items: Vec<_> = store.iter().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "file:///a.rs");
        assert_eq!(items[0].1.len(), 1);
    }

    #[test]
    fn store_diagnostics_at_line() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///test.rs".to_string(),
            vec![
                make_diagnostic(5, "err at 5", DiagnosticSeverity::Error),
                make_diagnostic(10, "err at 10", DiagnosticSeverity::Warning),
                make_diagnostic(5, "another at 5", DiagnosticSeverity::Hint),
            ],
        );

        let at_5 = store.diagnostics_at_line("file:///test.rs", 5);
        assert_eq!(at_5.len(), 2);

        let at_10 = store.diagnostics_at_line("file:///test.rs", 10);
        assert_eq!(at_10.len(), 1);

        let at_20 = store.diagnostics_at_line("file:///test.rs", 20);
        assert!(at_20.is_empty());
    }

    #[test]
    fn store_notify_on_update() {
        use std::sync::{Arc, Mutex};

        let updates = Arc::new(Mutex::new(Vec::new()));
        let updates_clone = updates.clone();

        let mut store = DiagnosticStore::new();
        store.set_on_update(move |uri, diags| {
            updates_clone
                .lock()
                .unwrap()
                .push((uri.to_string(), diags.len()));
        });

        store.publish(
            "file:///test.rs".to_string(),
            vec![make_diagnostic(0, "err", DiagnosticSeverity::Error)],
        );

        let captured = updates.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, "file:///test.rs");
        assert_eq!(captured[0].1, 1);
    }

    #[test]
    fn store_notify_on_clear() {
        use std::sync::{Arc, Mutex};

        let updates = Arc::new(Mutex::new(Vec::new()));
        let updates_clone = updates.clone();

        let mut store = DiagnosticStore::new();
        store.set_on_update(move |uri, diags| {
            updates_clone
                .lock()
                .unwrap()
                .push((uri.to_string(), diags.len()));
        });

        store.publish(
            "file:///test.rs".to_string(),
            vec![make_diagnostic(0, "err", DiagnosticSeverity::Error)],
        );
        // Clear by publishing empty
        store.publish("file:///test.rs".to_string(), vec![]);

        let captured = updates.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[1].1, 0); // Cleared
    }

    #[test]
    fn store_has_diagnostics_false_for_empty() {
        let store = DiagnosticStore::new();
        assert!(!store.has_diagnostics("file:///nothing.rs"));
    }

    #[test]
    fn store_multiple_diagnostics_same_file() {
        let mut store = DiagnosticStore::new();
        store.publish(
            "file:///test.rs".to_string(),
            vec![
                make_diagnostic(0, "first", DiagnosticSeverity::Error),
                make_diagnostic(1, "second", DiagnosticSeverity::Warning),
                make_diagnostic(2, "third", DiagnosticSeverity::Information),
            ],
        );
        assert_eq!(store.total_count(), 3);
        assert_eq!(store.file_count(), 1);
    }
}
