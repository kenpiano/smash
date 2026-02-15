use std::fmt;

/// A 0-based line/column position in a text buffer.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Hash,
)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    /// Create a new position at the given 0-based line and column.
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.line, self.col)
    }
}

/// A byte offset into a buffer.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Hash,
)]
pub struct ByteOffset(pub usize);

/// A range between two positions, guaranteed `start <= end`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Hash,
)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    /// Create a range, ensuring `start <= end`.
    pub fn new(a: Position, b: Position) -> Self {
        if a <= b {
            Self { start: a, end: b }
        } else {
            Self { start: b, end: a }
        }
    }

    /// Returns `true` if start == end.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns `true` if `pos` is inside the range (start inclusive,
    /// end exclusive).
    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Number of lines spanned (at least 1 if not empty, 0 if empty).
    pub fn len_lines(&self) -> usize {
        if self.is_empty() {
            return 0;
        }
        self.end.line - self.start.line + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Position tests ---

    #[test]
    fn position_new_creates_correct_values() {
        let p = Position::new(3, 7);
        assert_eq!(p.line, 3);
        assert_eq!(p.col, 7);
    }

    #[test]
    fn position_default_is_zero_zero() {
        let p = Position::default();
        assert_eq!(p, Position::new(0, 0));
    }

    #[test]
    fn position_display_format() {
        let p = Position::new(1, 2);
        assert_eq!(format!("{p}"), "(1, 2)");
    }

    #[test]
    fn position_ordering_line_first() {
        let a = Position::new(1, 5);
        let b = Position::new(2, 0);
        assert!(a < b);
    }

    #[test]
    fn position_ordering_col_within_same_line() {
        let a = Position::new(1, 3);
        let b = Position::new(1, 5);
        assert!(a < b);
    }

    #[test]
    fn position_equality() {
        let a = Position::new(4, 4);
        let b = Position::new(4, 4);
        assert_eq!(a, b);
    }

    #[test]
    fn position_clone_and_copy() {
        let a = Position::new(1, 2);
        let b = a;
        assert_eq!(a, b);
    }

    // --- ByteOffset tests ---

    #[test]
    fn byte_offset_default_is_zero() {
        assert_eq!(ByteOffset::default(), ByteOffset(0));
    }

    #[test]
    fn byte_offset_ordering() {
        assert!(ByteOffset(5) < ByteOffset(10));
    }

    // --- Range tests ---

    #[test]
    fn range_new_orders_positions() {
        let a = Position::new(5, 0);
        let b = Position::new(2, 3);
        let r = Range::new(a, b);
        assert_eq!(r.start, b);
        assert_eq!(r.end, a);
    }

    #[test]
    fn range_new_preserves_order_when_correct() {
        let a = Position::new(1, 0);
        let b = Position::new(3, 0);
        let r = Range::new(a, b);
        assert_eq!(r.start, a);
        assert_eq!(r.end, b);
    }

    #[test]
    fn range_is_empty_when_start_equals_end() {
        let p = Position::new(1, 1);
        let r = Range::new(p, p);
        assert!(r.is_empty());
    }

    #[test]
    fn range_is_not_empty_when_different() {
        let r = Range::new(Position::new(0, 0), Position::new(0, 1));
        assert!(!r.is_empty());
    }

    #[test]
    fn range_contains_start_inclusive() {
        let r = Range::new(Position::new(1, 0), Position::new(1, 5));
        assert!(r.contains(Position::new(1, 0)));
    }

    #[test]
    fn range_contains_end_exclusive() {
        let r = Range::new(Position::new(1, 0), Position::new(1, 5));
        assert!(!r.contains(Position::new(1, 5)));
    }

    #[test]
    fn range_contains_middle() {
        let r = Range::new(Position::new(0, 0), Position::new(2, 0));
        assert!(r.contains(Position::new(1, 5)));
    }

    #[test]
    fn range_does_not_contain_before() {
        let r = Range::new(Position::new(1, 0), Position::new(2, 0));
        assert!(!r.contains(Position::new(0, 5)));
    }

    #[test]
    fn range_len_lines_empty() {
        let r = Range::new(Position::new(3, 1), Position::new(3, 1));
        assert_eq!(r.len_lines(), 0);
    }

    #[test]
    fn range_len_lines_single_line() {
        let r = Range::new(Position::new(3, 0), Position::new(3, 5));
        assert_eq!(r.len_lines(), 1);
    }

    #[test]
    fn range_len_lines_multi_line() {
        let r = Range::new(Position::new(1, 0), Position::new(4, 0));
        assert_eq!(r.len_lines(), 4);
    }

    #[test]
    fn range_default_is_empty() {
        let r = Range::default();
        assert!(r.is_empty());
        assert_eq!(r.len_lines(), 0);
    }
}
