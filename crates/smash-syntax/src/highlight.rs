use crate::scope::ScopeId;

/// A highlighted span within a single line of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Byte offset of span start within line.
    pub start: usize,
    /// Byte offset of span end within line (exclusive).
    pub end: usize,
    /// The semantic scope of this span.
    pub scope: ScopeId,
}

impl HighlightSpan {
    /// Create a new highlight span.
    pub fn new(start: usize, end: usize, scope: ScopeId) -> Self {
        Self { start, end, scope }
    }

    /// Byte length of the span.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span has zero length.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Trait for syntax highlight engines.
///
/// Phase 1 uses [`crate::RegexHighlighter`]; later this can be
/// swapped for a tree-sitter implementation.
pub trait HighlightEngine: Send + Sync {
    /// Highlight a single line of text, returning spans.
    fn highlight_line(&self, line: &str) -> Vec<HighlightSpan>;

    /// Name of the engine (for logging / diagnostics).
    fn engine_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_new_sets_fields() {
        let span = HighlightSpan::new(0, 5, ScopeId::Keyword);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        assert_eq!(span.scope, ScopeId::Keyword);
    }

    #[test]
    fn span_len_basic() {
        let span = HighlightSpan::new(3, 10, ScopeId::String);
        assert_eq!(span.len(), 7);
    }

    #[test]
    fn span_len_zero() {
        let span = HighlightSpan::new(5, 5, ScopeId::Comment);
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn span_len_saturates_when_start_exceeds_end() {
        let span = HighlightSpan::new(10, 3, ScopeId::Number);
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn span_is_empty_when_zero_length() {
        let span = HighlightSpan::new(5, 5, ScopeId::Plain);
        assert!(span.is_empty());
    }

    #[test]
    fn span_is_not_empty_when_positive_length() {
        let span = HighlightSpan::new(0, 1, ScopeId::Operator);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_clone_and_eq() {
        let a = HighlightSpan::new(0, 5, ScopeId::Type);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn span_debug() {
        let span = HighlightSpan::new(0, 3, ScopeId::Comment);
        let debug = format!("{:?}", span);
        assert!(debug.contains("HighlightSpan"));
        assert!(debug.contains("Comment"));
    }

    #[test]
    fn span_ne_different_scope() {
        let a = HighlightSpan::new(0, 5, ScopeId::Keyword);
        let b = HighlightSpan::new(0, 5, ScopeId::String);
        assert_ne!(a, b);
    }

    #[test]
    fn span_ne_different_range() {
        let a = HighlightSpan::new(0, 5, ScopeId::Keyword);
        let b = HighlightSpan::new(0, 6, ScopeId::Keyword);
        assert_ne!(a, b);
    }
}
