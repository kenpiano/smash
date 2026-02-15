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
}
