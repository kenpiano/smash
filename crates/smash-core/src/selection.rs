use crate::position::{Position, Range};

/// A directional selection: `anchor` is where the selection started,
/// `head` is where the cursor currently is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    anchor: Position,
    head: Position,
}

impl Selection {
    /// Create a new selection from `anchor` to `head`.
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    /// The anchor position.
    pub fn anchor(&self) -> Position {
        self.anchor
    }

    /// The head (cursor) position.
    pub fn head(&self) -> Position {
        self.head
    }

    /// The range of the selection (sorted).
    pub fn range(&self) -> Range {
        Range::new(self.anchor, self.head)
    }

    /// Returns `true` if the selection is forward (anchor <= head).
    pub fn is_forward(&self) -> bool {
        self.anchor <= self.head
    }
}

/// A non-overlapping set of selections, sorted by start position.
#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    selections: Vec<Selection>,
}

impl SelectionSet {
    /// Create an empty selection set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a selection and re-normalize.
    pub fn push(&mut self, sel: Selection) {
        self.selections.push(sel);
        self.normalize();
    }

    /// Iterate over selections.
    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }

    /// Number of selections.
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Returns `true` if there are no selections.
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Sort by start position and merge overlapping selections.
    fn normalize(&mut self) {
        // Sort by the start of each selection's range
        self.selections
            .sort_by(|a, b| a.range().start.cmp(&b.range().start));

        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for sel in self.selections.drain(..) {
            if let Some(last) = merged.last() {
                let last_range = last.range();
                let sel_range = sel.range();
                if sel_range.start <= last_range.end {
                    // Overlapping: merge
                    let idx = merged.len() - 1;
                    let new_start = last_range.start;
                    let new_end = if sel_range.end > last_range.end {
                        sel_range.end
                    } else {
                        last_range.end
                    };
                    merged[idx] = Selection::new(new_start, new_end);
                    continue;
                }
            }
            merged.push(sel);
        }
        self.selections = merged;
    }
}

/// A rectangular (column/block) selection defined by two corner positions.
/// The selection covers all lines from `start.line` to `end.line`,
/// and columns from `min(start.col, end.col)` to `max(start.col, end.col)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSelection {
    anchor: Position,
    head: Position,
}

impl ColumnSelection {
    /// Create a new column selection from `anchor` to `head`.
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    /// The anchor position.
    pub fn anchor(&self) -> Position {
        self.anchor
    }

    /// The head (cursor) position.
    pub fn head(&self) -> Position {
        self.head
    }

    /// The range of lines covered by this column selection (sorted).
    pub fn line_range(&self) -> (usize, usize) {
        let start = self.anchor.line.min(self.head.line);
        let end = self.anchor.line.max(self.head.line);
        (start, end)
    }

    /// The column range covered (sorted).
    pub fn col_range(&self) -> (usize, usize) {
        let start = self.anchor.col.min(self.head.col);
        let end = self.anchor.col.max(self.head.col);
        (start, end)
    }

    /// Convert to a set of per-line selections.
    /// Each line gets a Selection from (line, col_start) to (line, col_end).
    /// Lines shorter than col_start are skipped.
    /// Lines shorter than col_end have their selection truncated.
    pub fn to_line_selections(&self, line_lengths: &[usize]) -> Vec<Selection> {
        let (line_start, line_end) = self.line_range();
        let (col_start, col_end) = self.col_range();
        let mut result = Vec::new();
        for line in line_start..=line_end {
            let len = line_lengths.get(line).copied().unwrap_or(0);
            if len <= col_start {
                // Line is too short — skip
                continue;
            }
            let actual_end = col_end.min(len);
            result.push(Selection::new(
                Position::new(line, col_start),
                Position::new(line, actual_end),
            ));
        }
        result
    }

    /// Number of lines in the selection.
    pub fn height(&self) -> usize {
        let (start, end) = self.line_range();
        end - start + 1
    }

    /// Width of the selection in columns.
    pub fn width(&self) -> usize {
        let (start, end) = self.col_range();
        end - start
    }

    /// Check if a position is inside the rectangular selection.
    pub fn contains(&self, pos: Position) -> bool {
        let (line_start, line_end) = self.line_range();
        let (col_start, col_end) = self.col_range();
        pos.line >= line_start && pos.line <= line_end && pos.col >= col_start && pos.col < col_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_is_forward() {
        let sel = Selection::new(Position::new(1, 0), Position::new(3, 0));
        assert!(sel.is_forward());
    }

    #[test]
    fn selection_is_backward() {
        let sel = Selection::new(Position::new(3, 0), Position::new(1, 0));
        assert!(!sel.is_forward());
    }

    #[test]
    fn selection_range_is_sorted() {
        let sel = Selection::new(Position::new(5, 0), Position::new(2, 0));
        let r = sel.range();
        assert_eq!(r.start, Position::new(2, 0));
        assert_eq!(r.end, Position::new(5, 0));
    }

    #[test]
    fn selection_accessors() {
        let sel = Selection::new(Position::new(1, 2), Position::new(3, 4));
        assert_eq!(sel.anchor(), Position::new(1, 2));
        assert_eq!(sel.head(), Position::new(3, 4));
    }

    #[test]
    fn selection_set_empty() {
        let ss = SelectionSet::new();
        assert!(ss.is_empty());
        assert_eq!(ss.len(), 0);
    }

    #[test]
    fn selection_set_push() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(0, 0), Position::new(0, 5)));
        assert_eq!(ss.len(), 1);
    }

    #[test]
    fn selection_set_sorts_by_start() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(5, 0), Position::new(6, 0)));
        ss.push(Selection::new(Position::new(1, 0), Position::new(2, 0)));
        let starts: Vec<Position> = ss.iter().map(|s| s.range().start).collect();
        assert_eq!(starts, vec![Position::new(1, 0), Position::new(5, 0)]);
    }

    #[test]
    fn selection_set_merges_overlapping() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(0, 0), Position::new(0, 5)));
        ss.push(Selection::new(Position::new(0, 3), Position::new(0, 8)));
        // Should merge into one selection covering 0..8
        assert_eq!(ss.len(), 1);
        let r = ss.iter().next().unwrap().range();
        assert_eq!(r.start, Position::new(0, 0));
        assert_eq!(r.end, Position::new(0, 8));
    }

    #[test]
    fn selection_set_no_merge_non_overlapping() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(0, 0), Position::new(0, 3)));
        ss.push(Selection::new(Position::new(0, 5), Position::new(0, 8)));
        assert_eq!(ss.len(), 2);
    }

    #[test]
    fn selection_set_merges_adjacent() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(0, 0), Position::new(0, 5)));
        ss.push(Selection::new(Position::new(0, 5), Position::new(0, 10)));
        // Adjacent (touching at boundary) should merge
        assert_eq!(ss.len(), 1);
        let r = ss.iter().next().unwrap().range();
        assert_eq!(r.start, Position::new(0, 0));
        assert_eq!(r.end, Position::new(0, 10));
    }

    #[test]
    fn selection_set_iter() {
        let mut ss = SelectionSet::new();
        ss.push(Selection::new(Position::new(0, 0), Position::new(0, 1)));
        ss.push(Selection::new(Position::new(1, 0), Position::new(1, 1)));
        assert_eq!(ss.iter().count(), 2);
    }

    // --- ColumnSelection tests ---

    #[test]
    fn column_selection_new() {
        let cs = ColumnSelection::new(Position::new(1, 2), Position::new(4, 6));
        assert_eq!(cs.anchor(), Position::new(1, 2));
        assert_eq!(cs.head(), Position::new(4, 6));
    }

    #[test]
    fn column_selection_line_range() {
        // Forward
        let cs = ColumnSelection::new(Position::new(2, 0), Position::new(5, 0));
        assert_eq!(cs.line_range(), (2, 5));
        // Backward (anchor below head)
        let cs2 = ColumnSelection::new(Position::new(5, 0), Position::new(2, 0));
        assert_eq!(cs2.line_range(), (2, 5));
    }

    #[test]
    fn column_selection_col_range() {
        let cs = ColumnSelection::new(Position::new(0, 10), Position::new(3, 3));
        assert_eq!(cs.col_range(), (3, 10));
        let cs2 = ColumnSelection::new(Position::new(0, 3), Position::new(3, 10));
        assert_eq!(cs2.col_range(), (3, 10));
    }

    #[test]
    fn column_selection_height_and_width() {
        let cs = ColumnSelection::new(Position::new(2, 5), Position::new(6, 10));
        assert_eq!(cs.height(), 5); // lines 2,3,4,5,6
        assert_eq!(cs.width(), 5); // cols 5..10
    }

    #[test]
    fn column_selection_contains() {
        let cs = ColumnSelection::new(Position::new(1, 3), Position::new(4, 7));
        // Inside
        assert!(cs.contains(Position::new(2, 5)));
        // On start boundary (inclusive)
        assert!(cs.contains(Position::new(1, 3)));
        // On end col boundary (exclusive)
        assert!(!cs.contains(Position::new(2, 7)));
        // Outside line range
        assert!(!cs.contains(Position::new(0, 5)));
        assert!(!cs.contains(Position::new(5, 5)));
        // Outside col range
        assert!(!cs.contains(Position::new(2, 2)));
        assert!(!cs.contains(Position::new(2, 8)));
    }

    #[test]
    fn column_selection_to_line_selections_basic() {
        // 3 lines, all long enough
        let cs = ColumnSelection::new(Position::new(0, 2), Position::new(2, 5));
        let line_lengths = vec![10, 10, 10];
        let sels = cs.to_line_selections(&line_lengths);
        assert_eq!(sels.len(), 3);
        assert_eq!(
            sels[0],
            Selection::new(Position::new(0, 2), Position::new(0, 5))
        );
        assert_eq!(
            sels[1],
            Selection::new(Position::new(1, 2), Position::new(1, 5))
        );
        assert_eq!(
            sels[2],
            Selection::new(Position::new(2, 2), Position::new(2, 5))
        );
    }

    #[test]
    fn column_selection_to_line_selections_short_lines() {
        // line 0: length 10 => full selection
        // line 1: length 3  => col_start=4 > 3, skip
        // line 2: length 6  => col_start=4 ok, col_end truncated from 8 to 6
        let cs = ColumnSelection::new(Position::new(0, 4), Position::new(2, 8));
        let line_lengths = vec![10, 3, 6];
        let sels = cs.to_line_selections(&line_lengths);
        assert_eq!(sels.len(), 2);
        assert_eq!(
            sels[0],
            Selection::new(Position::new(0, 4), Position::new(0, 8))
        );
        assert_eq!(
            sels[1],
            Selection::new(Position::new(2, 4), Position::new(2, 6))
        );
    }

    #[test]
    fn selection_column_rect() {
        // Verify column selection works with reversed anchor/head
        let cs = ColumnSelection::new(Position::new(5, 10), Position::new(2, 3));
        assert_eq!(cs.line_range(), (2, 5));
        assert_eq!(cs.col_range(), (3, 10));
        assert_eq!(cs.height(), 4);
        assert_eq!(cs.width(), 7);
        assert!(cs.contains(Position::new(3, 5)));
        assert!(!cs.contains(Position::new(3, 10)));

        // to_line_selections with reversed anchor/head
        let line_lengths = vec![0, 0, 20, 5, 20, 20];
        let sels = cs.to_line_selections(&line_lengths);
        // line 2: full, line 3: col_start=3 < 5, truncated end to 5
        // line 4: full, line 5: full
        assert_eq!(sels.len(), 4);
        assert_eq!(
            sels[0],
            Selection::new(Position::new(2, 3), Position::new(2, 10))
        );
        assert_eq!(
            sels[1],
            Selection::new(Position::new(3, 3), Position::new(3, 5))
        );
        assert_eq!(
            sels[2],
            Selection::new(Position::new(4, 3), Position::new(4, 10))
        );
        assert_eq!(
            sels[3],
            Selection::new(Position::new(5, 3), Position::new(5, 10))
        );
    }
}
