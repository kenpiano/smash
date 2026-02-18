use crate::position::{Position, Range};

/// A cursor in a text buffer, optionally with a selection anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    position: Position,
    anchor: Option<Position>,
}

impl Cursor {
    /// Create a cursor at `pos` with no selection.
    pub fn new(pos: Position) -> Self {
        Self {
            position: pos,
            anchor: None,
        }
    }

    /// Create a cursor at `pos` with selection anchored at `anchor`.
    pub fn with_selection(pos: Position, anchor: Position) -> Self {
        Self {
            position: pos,
            anchor: Some(anchor),
        }
    }

    /// The current cursor position (caret).
    pub fn position(&self) -> Position {
        self.position
    }

    /// The selection anchor, if any.
    pub fn anchor(&self) -> Option<Position> {
        self.anchor
    }

    /// Set the cursor position.
    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    /// Returns `true` if a selection is active.
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some()
    }

    /// Returns the selection range (sorted start..end), or `None`.
    pub fn selection_range(&self) -> Option<Range> {
        self.anchor.map(|a| Range::new(a, self.position))
    }

    /// Clear any active selection, keeping the cursor position.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }
}

/// An ordered set of cursors, kept sorted by position.
/// The first cursor is always the primary cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSet {
    cursors: Vec<Cursor>,
}

impl CursorSet {
    /// Create a cursor set with a single primary cursor.
    pub fn new(primary: Cursor) -> Self {
        Self {
            cursors: vec![primary],
        }
    }

    /// Reference to the primary (first) cursor.
    pub fn primary(&self) -> &Cursor {
        &self.cursors[0]
    }

    /// Mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[0]
    }

    /// Add a cursor, keeping the set sorted by position.
    /// Merges overlapping selections.
    pub fn add(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.normalize();
    }

    /// Number of cursors.
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Returns `true` if there are no cursors (never true in practice).
    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// Iterate over all cursors.
    pub fn iter(&self) -> impl Iterator<Item = &Cursor> {
        self.cursors.iter()
    }

    /// Remove all secondary cursors, keeping only the primary.
    pub fn clear_secondary(&mut self) {
        self.cursors.truncate(1);
    }

    /// Shift cursor positions after an edit that changed a region from
    /// `edit_pos..old_end` to `edit_pos..new_end`.
    pub fn remap_after_edit(&mut self, edit_pos: Position, old_end: Position, new_end: Position) {
        for cursor in &mut self.cursors {
            cursor.position = remap_position(cursor.position, edit_pos, old_end, new_end);
            if let Some(anchor) = cursor.anchor {
                cursor.anchor = Some(remap_position(anchor, edit_pos, old_end, new_end));
            }
        }
    }

    /// Find the next occurrence of `pattern` after the last cursor and add a new cursor there.
    /// Returns true if a new cursor was added.
    pub fn add_cursor_at_next_match(&mut self, text: &ropey::Rope, pattern: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }

        let text_str = text.to_string();

        // Find the byte offset just after the last cursor's position (or end of its selection)
        let last_cursor = self
            .cursors
            .iter()
            .max_by_key(|c| c.selection_range().map_or(c.position(), |r| r.end))
            .expect("CursorSet always has at least one cursor");
        let search_from = last_cursor
            .selection_range()
            .map_or(last_cursor.position(), |r| r.end);
        let search_line = search_from.line.min(text.len_lines() - 1);
        let search_line_start = text.line_to_char(search_line);
        let search_char_idx = (search_line_start + search_from.col).min(text.len_chars());
        // Start one character past the last cursor/selection end to avoid re-matching
        let search_char_idx = (search_char_idx + 1).min(text.len_chars());
        let search_byte = text.char_to_byte(search_char_idx);

        // Search forward from after the last cursor
        let found = text_str[search_byte..]
            .find(pattern)
            .map(|offset| search_byte + offset);

        // If not found, wrap around from the beginning
        let found = found.or_else(|| text_str[..search_byte].find(pattern));

        match found {
            Some(byte_offset) => {
                let start_char = text.byte_to_char(byte_offset);
                let end_char = start_char + pattern.chars().count();
                let start_line = text.char_to_line(start_char);
                let start_col = start_char - text.line_to_char(start_line);
                let end_line = text.char_to_line(end_char);
                let end_col = end_char - text.line_to_char(end_line);

                let start_pos = Position::new(start_line, start_col);
                let end_pos = Position::new(end_line, end_col);

                // Check that we don't already have a cursor with this exact selection
                let already_exists = self.cursors.iter().any(|c| {
                    c.selection_range()
                        .is_some_and(|r| r.start == start_pos && r.end == end_pos)
                });
                if already_exists {
                    return false;
                }

                self.add(Cursor::with_selection(end_pos, start_pos));
                true
            }
            None => false,
        }
    }

    /// Sort by position and merge overlapping cursors.
    fn normalize(&mut self) {
        // Sort by position
        self.cursors.sort_by_key(|a| a.position());

        // Merge overlapping selections
        let mut merged: Vec<Cursor> = Vec::with_capacity(self.cursors.len());
        for cursor in self.cursors.drain(..) {
            if let Some(last) = merged.last() {
                if Self::overlaps(last, &cursor) {
                    // Merge: keep the one with the later position
                    let idx = merged.len() - 1;
                    let existing = &merged[idx];
                    let merged_cursor = Self::merge_cursors(existing, &cursor);
                    merged[idx] = merged_cursor;
                    continue;
                }
            }
            merged.push(cursor);
        }
        self.cursors = merged;
    }

    fn overlaps(a: &Cursor, b: &Cursor) -> bool {
        // Two cursors overlap if their positions are the same
        // or their selection ranges overlap
        if a.position() == b.position() {
            return true;
        }
        match (a.selection_range(), b.selection_range()) {
            (Some(ra), Some(rb)) => ra.start < rb.end && rb.start < ra.end,
            _ => false,
        }
    }

    fn merge_cursors(a: &Cursor, b: &Cursor) -> Cursor {
        match (a.selection_range(), b.selection_range()) {
            (Some(ra), Some(rb)) => {
                let start = if ra.start < rb.start {
                    ra.start
                } else {
                    rb.start
                };
                let end = if ra.end > rb.end { ra.end } else { rb.end };
                Cursor::with_selection(end, start)
            }
            _ => {
                // Keep the later position
                if b.position() >= a.position() {
                    b.clone()
                } else {
                    a.clone()
                }
            }
        }
    }
}

/// Remap a position after an edit that replaced
/// `edit_pos..old_end` with `edit_pos..new_end`.
fn remap_position(
    pos: Position,
    edit_pos: Position,
    old_end: Position,
    new_end: Position,
) -> Position {
    if pos <= edit_pos {
        // Before the edit: unchanged
        pos
    } else if pos >= old_end {
        // After the old edit region: shift by the delta
        let line_delta = new_end.line as isize - old_end.line as isize;
        let new_line = (pos.line as isize + line_delta).max(0) as usize;
        let new_col = if pos.line == old_end.line {
            // Same line as old_end: adjust column relative to new_end
            let col_delta = new_end.col as isize - old_end.col as isize;
            (pos.col as isize + col_delta).max(0) as usize
        } else {
            pos.col
        };
        Position::new(new_line, new_col)
    } else {
        // Inside the deleted region: clamp to new_end
        new_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Cursor tests ---

    #[test]
    fn cursor_new_no_selection() {
        let c = Cursor::new(Position::new(1, 2));
        assert_eq!(c.position(), Position::new(1, 2));
        assert!(!c.has_selection());
        assert!(c.selection_range().is_none());
    }

    #[test]
    fn cursor_with_selection() {
        let c = Cursor::with_selection(Position::new(3, 0), Position::new(1, 0));
        assert!(c.has_selection());
        let range = c.selection_range().unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(3, 0));
    }

    #[test]
    fn cursor_with_selection_reversed() {
        let c = Cursor::with_selection(Position::new(1, 0), Position::new(3, 0));
        let range = c.selection_range().unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(3, 0));
    }

    #[test]
    fn cursor_clear_selection() {
        let mut c = Cursor::with_selection(Position::new(3, 0), Position::new(1, 0));
        c.clear_selection();
        assert!(!c.has_selection());
        assert_eq!(c.position(), Position::new(3, 0));
    }

    #[test]
    fn cursor_set_position() {
        let mut c = Cursor::new(Position::new(0, 0));
        c.set_position(Position::new(5, 3));
        assert_eq!(c.position(), Position::new(5, 3));
    }

    // --- CursorSet tests ---

    #[test]
    fn cursor_set_new_has_one_cursor() {
        let cs = CursorSet::new(Cursor::new(Position::new(0, 0)));
        assert_eq!(cs.len(), 1);
        assert!(!cs.is_empty());
    }

    #[test]
    fn cursor_set_primary() {
        let cs = CursorSet::new(Cursor::new(Position::new(2, 3)));
        assert_eq!(cs.primary().position(), Position::new(2, 3));
    }

    #[test]
    fn cursor_set_add_keeps_sorted() {
        let mut cs = CursorSet::new(Cursor::new(Position::new(5, 0)));
        cs.add(Cursor::new(Position::new(2, 0)));
        cs.add(Cursor::new(Position::new(8, 0)));

        let positions: Vec<Position> = cs.iter().map(|c| c.position()).collect();
        assert_eq!(
            positions,
            vec![
                Position::new(2, 0),
                Position::new(5, 0),
                Position::new(8, 0),
            ]
        );
    }

    #[test]
    fn cursor_set_merges_same_position() {
        let mut cs = CursorSet::new(Cursor::new(Position::new(3, 0)));
        cs.add(Cursor::new(Position::new(3, 0)));
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn cursor_set_merges_overlapping_selections() {
        let mut cs = CursorSet::new(Cursor::with_selection(
            Position::new(3, 0),
            Position::new(1, 0),
        ));
        cs.add(Cursor::with_selection(
            Position::new(4, 0),
            Position::new(2, 0),
        ));
        // Should merge into a single cursor covering 1..4
        assert_eq!(cs.len(), 1);
        let range = cs.primary().selection_range().unwrap();
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(4, 0));
    }

    #[test]
    fn cursor_set_clear_secondary() {
        let mut cs = CursorSet::new(Cursor::new(Position::new(0, 0)));
        cs.add(Cursor::new(Position::new(5, 0)));
        cs.add(Cursor::new(Position::new(10, 0)));
        assert_eq!(cs.len(), 3);
        cs.clear_secondary();
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn cursor_set_primary_mut() {
        let mut cs = CursorSet::new(Cursor::new(Position::new(0, 0)));
        cs.primary_mut().set_position(Position::new(10, 5));
        assert_eq!(cs.primary().position(), Position::new(10, 5));
    }

    // --- Remap tests ---

    #[test]
    fn remap_position_before_edit_unchanged() {
        let pos = Position::new(0, 5);
        let result = remap_position(
            pos,
            Position::new(1, 0),
            Position::new(1, 3),
            Position::new(1, 6),
        );
        assert_eq!(result, pos);
    }

    #[test]
    fn remap_position_after_edit_shifted() {
        // Edit replaced line 1 col 0..3 with 6 chars (same line)
        let pos = Position::new(1, 5);
        let result = remap_position(
            pos,
            Position::new(1, 0),
            Position::new(1, 3),
            Position::new(1, 6),
        );
        // col should shift by +3
        assert_eq!(result, Position::new(1, 8));
    }

    #[test]
    fn remap_position_inside_deleted_region() {
        let pos = Position::new(1, 2);
        let result = remap_position(
            pos,
            Position::new(1, 0),
            Position::new(1, 5),
            Position::new(1, 0),
        );
        // Should clamp to new_end
        assert_eq!(result, Position::new(1, 0));
    }

    #[test]
    fn remap_position_after_multiline_insert() {
        // Insert added 2 new lines at (1, 0)
        let pos = Position::new(3, 5);
        let result = remap_position(
            pos,
            Position::new(1, 0),
            Position::new(1, 0),
            Position::new(3, 0),
        );
        // line shifts by +2
        assert_eq!(result, Position::new(5, 5));
    }

    #[test]
    fn cursor_set_remap_after_edit() {
        let mut cs = CursorSet::new(Cursor::new(Position::new(2, 5)));
        cs.remap_after_edit(
            Position::new(1, 0),
            Position::new(1, 3),
            Position::new(1, 6),
        );
        // Cursor on line 2 should be unchanged (different line)
        assert_eq!(cs.primary().position(), Position::new(2, 5));
    }

    #[test]
    fn cursor_set_remap_with_selection() {
        let mut cs = CursorSet::new(Cursor::with_selection(
            Position::new(1, 10),
            Position::new(1, 5),
        ));
        // Insert 3 chars at (1, 0)
        cs.remap_after_edit(
            Position::new(1, 0),
            Position::new(1, 0),
            Position::new(1, 3),
        );
        assert_eq!(cs.primary().position(), Position::new(1, 13));
        assert_eq!(cs.primary().anchor().unwrap(), Position::new(1, 8));
    }

    // --- add_cursor_at_next_match tests ---

    #[test]
    fn add_cursor_at_next_match_finds_match() {
        let rope = ropey::Rope::from_str("hello world hello rust");
        let mut cs = CursorSet::new(Cursor::new(Position::new(0, 0)));
        let found = cs.add_cursor_at_next_match(&rope, "hello");
        assert!(found);
        assert_eq!(cs.len(), 2);
        // The new cursor should have a selection covering the match
        let second = cs.iter().nth(1).unwrap();
        let range = second.selection_range().unwrap();
        assert_eq!(range.start, Position::new(0, 12));
        assert_eq!(range.end, Position::new(0, 17));
    }

    #[test]
    fn add_cursor_at_next_match_no_match_returns_false() {
        let rope = ropey::Rope::from_str("hello world");
        let mut cs = CursorSet::new(Cursor::new(Position::new(0, 5)));
        let found = cs.add_cursor_at_next_match(&rope, "xyz");
        assert!(!found);
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn add_cursor_at_next_match_wraps_around() {
        // Cursor is past the only occurrence — should wrap around
        let rope = ropey::Rope::from_str("hello world");
        let mut cs = CursorSet::new(Cursor::new(Position::new(0, 8)));
        let found = cs.add_cursor_at_next_match(&rope, "hello");
        assert!(found);
        assert_eq!(cs.len(), 2);
        // The wrapped-around match starts at col 0
        let cursors: Vec<_> = cs.iter().collect();
        let new_cursor = cursors[0]; // sorted by position, so (0,0) first
        let range = new_cursor.selection_range().unwrap();
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 5));
    }
}
