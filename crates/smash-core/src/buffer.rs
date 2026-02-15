use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ropey::Rope;

use crate::cursor::{Cursor, CursorSet};
use crate::edit::{EditCommand, EditEvent};
use crate::encoding::{detect_line_ending, LineEnding};
use crate::error::EditError;
use crate::position::{Position, Range};
use crate::search::SearchState;
use crate::undo::UndoTree;

/// Global counter for generating unique buffer IDs.
static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BufferId(pub u64);

impl BufferId {
    /// Generate a fresh, unique `BufferId`.
    pub fn next() -> Self {
        Self(NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A text buffer backed by a rope data structure.
#[derive(Debug)]
pub struct Buffer {
    id: BufferId,
    rope: Rope,
    path: Option<PathBuf>,
    dirty: bool,
    line_ending: LineEnding,
    undo_tree: UndoTree,
    cursors: CursorSet,
    search: SearchState,
}

impl Buffer {
    /// Create a new empty buffer.
    pub fn new(id: BufferId) -> Self {
        Self {
            id,
            rope: Rope::new(),
            path: None,
            dirty: false,
            line_ending: LineEnding::default(),
            undo_tree: UndoTree::new(),
            cursors: CursorSet::new(Cursor::new(Position::default())),
            search: SearchState::new(),
        }
    }

    /// Create a buffer from a text string.
    pub fn from_text(id: BufferId, text: &str) -> Self {
        let line_ending = detect_line_ending(text);
        Self {
            id,
            rope: Rope::from_str(text),
            path: None,
            dirty: false,
            line_ending,
            undo_tree: UndoTree::new(),
            cursors: CursorSet::new(Cursor::new(Position::default())),
            search: SearchState::new(),
        }
    }

    /// Create a buffer by reading a file from disk.
    pub fn from_file(id: BufferId, path: &Path) -> Result<Self, EditError> {
        if !path.exists() {
            return Err(EditError::FileNotFound(path.to_path_buf()));
        }
        let text = std::fs::read_to_string(path)?;
        let line_ending = detect_line_ending(&text);
        Ok(Self {
            id,
            rope: Rope::from_str(&text),
            path: Some(path.to_path_buf()),
            dirty: false,
            line_ending,
            undo_tree: UndoTree::new(),
            cursors: CursorSet::new(Cursor::new(Position::default())),
            search: SearchState::new(),
        })
    }

    /// Open a file if it exists, or create an empty buffer with the
    /// path set if it does not.
    ///
    /// This is the primary entry point for the "open file" command.
    /// When the file does not exist on disk, an empty buffer is
    /// created and associated with the given path so that a
    /// subsequent `save()` will write to that location.
    pub fn open_or_create(id: BufferId, path: &Path) -> Result<Self, EditError> {
        if path.exists() {
            Self::from_file(id, path)
        } else {
            Ok(Self {
                id,
                rope: Rope::new(),
                path: Some(path.to_path_buf()),
                dirty: false,
                line_ending: LineEnding::default(),
                undo_tree: UndoTree::new(),
                cursors: CursorSet::new(Cursor::new(Position::default())),
                search: SearchState::new(),
            })
        }
    }

    /// Save buffer contents to the associated file path.
    pub fn save(&mut self) -> Result<(), EditError> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| EditError::FileNotFound(PathBuf::from("<no path>")))?;
        self.save_as(&path)
    }

    /// Save buffer contents to a specific path.
    pub fn save_as(&mut self, path: &Path) -> Result<(), EditError> {
        let text = self.rope.to_string();
        std::fs::write(path, &text)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    /// Reference to the underlying rope.
    pub fn text(&self) -> &Rope {
        &self.rope
    }

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get a line by 0-based index.
    pub fn line(&self, idx: usize) -> Option<ropey::RopeSlice<'_>> {
        if idx < self.rope.len_lines() {
            Some(self.rope.line(idx))
        } else {
            None
        }
    }

    /// Total bytes in the buffer.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Total chars in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Whether the buffer has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The file path associated with this buffer, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The buffer's unique ID.
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// The detected or configured line ending.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Reference to the cursor set.
    pub fn cursors(&self) -> &CursorSet {
        &self.cursors
    }

    /// Mutable reference to the cursor set.
    pub fn cursors_mut(&mut self) -> &mut CursorSet {
        &mut self.cursors
    }

    /// Reference to the search state.
    pub fn search(&self) -> &SearchState {
        &self.search
    }

    /// Mutable reference to the search state.
    pub fn search_mut(&mut self) -> &mut SearchState {
        &mut self.search
    }

    /// Apply an edit command and return the resulting edit events.
    pub fn apply_edit(&mut self, cmd: EditCommand) -> Result<Vec<EditEvent>, EditError> {
        let cursor_before = self.cursors.primary().position();
        let (events, inverse) = self.apply_edit_inner(&cmd)?;
        self.undo_tree.record(inverse, cmd, cursor_before);
        self.dirty = true;
        Ok(events)
    }

    /// Undo the last edit.
    pub fn undo(&mut self) -> Result<Option<Vec<EditEvent>>, EditError> {
        match self.undo_tree.undo() {
            Some((inverse_cmd, cursor_pos)) => {
                let (events, re_inverse) = self.apply_edit_inner(&inverse_cmd)?;
                // The re_inverse is the redo operation; it's already
                // in the tree as the node we just undid from.
                let _ = re_inverse;
                self.cursors.primary_mut().set_position(cursor_pos);
                Ok(Some(events))
            }
            None => Ok(None),
        }
    }

    /// Redo the last undone edit.
    pub fn redo(&mut self) -> Result<Option<Vec<EditEvent>>, EditError> {
        match self.undo_tree.redo() {
            Some((forward_cmd, _cursor_pos)) => {
                // The forward op is the original edit; apply it.
                let (events, _inverse) = self.apply_edit_inner(&forward_cmd)?;
                Ok(Some(events))
            }
            None => Ok(None),
        }
    }

    /// Convert a (line, col) position to a char index in the rope.
    fn position_to_char_idx(&self, pos: Position) -> Result<usize, EditError> {
        let line_count = self.rope.len_lines();
        if pos.line >= line_count {
            return Err(EditError::OutOfBounds(pos));
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.rope.line(pos.line).len_chars();
        if pos.col > line_len {
            return Err(EditError::OutOfBounds(pos));
        }
        Ok(line_start + pos.col)
    }

    /// Convert a char index back to a Position.
    fn char_idx_to_position(&self, char_idx: usize) -> Position {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let col = char_idx - line_start;
        Position::new(line, col)
    }

    /// Apply an edit without recording in the undo tree.
    /// Returns (events, inverse_command).
    fn apply_edit_inner(
        &mut self,
        cmd: &EditCommand,
    ) -> Result<(Vec<EditEvent>, EditCommand), EditError> {
        match cmd {
            EditCommand::Insert { pos, text } => {
                let char_idx = self.position_to_char_idx(*pos)?;
                let start_byte = self.rope.char_to_byte(char_idx);
                let old_end_byte = start_byte;

                self.rope.insert(char_idx, text);

                let new_char_idx = char_idx + text.chars().count();
                let new_end_byte = self.rope.char_to_byte(new_char_idx);
                let new_end_pos = self.char_idx_to_position(new_char_idx);

                let event = EditEvent {
                    start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: *pos,
                    old_end_position: *pos,
                    new_end_position: new_end_pos,
                };

                let inverse = EditCommand::Delete {
                    range: Range::new(*pos, new_end_pos),
                };

                Ok((vec![event], inverse))
            }

            EditCommand::Delete { range } => {
                let start_idx = self.position_to_char_idx(range.start)?;
                let end_idx = self.position_to_char_idx(range.end)?;

                let start_byte = self.rope.char_to_byte(start_idx);
                let old_end_byte = self.rope.char_to_byte(end_idx);

                let deleted_text: String = self.rope.slice(start_idx..end_idx).into();

                self.rope.remove(start_idx..end_idx);

                let new_end_byte = start_byte;

                let event = EditEvent {
                    start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: range.start,
                    old_end_position: range.end,
                    new_end_position: range.start,
                };

                let inverse = EditCommand::Insert {
                    pos: range.start,
                    text: deleted_text,
                };

                Ok((vec![event], inverse))
            }

            EditCommand::Replace { range, text } => {
                let start_idx = self.position_to_char_idx(range.start)?;
                let end_idx = self.position_to_char_idx(range.end)?;

                let start_byte = self.rope.char_to_byte(start_idx);
                let old_end_byte = self.rope.char_to_byte(end_idx);

                let old_text: String = self.rope.slice(start_idx..end_idx).into();

                self.rope.remove(start_idx..end_idx);
                self.rope.insert(start_idx, text);

                let new_char_idx = start_idx + text.chars().count();
                let new_end_byte = self.rope.char_to_byte(new_char_idx);
                let new_end_pos = self.char_idx_to_position(new_char_idx);

                let event = EditEvent {
                    start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: range.start,
                    old_end_position: range.end,
                    new_end_position: new_end_pos,
                };

                let new_range = Range::new(range.start, new_end_pos);
                let inverse = EditCommand::Replace {
                    range: new_range,
                    text: old_text,
                };

                Ok((vec![event], inverse))
            }

            EditCommand::IndentLines { lines, direction } => {
                let mut all_events = Vec::new();
                let indent_str = "    "; // 4 spaces
                let mut inverse_lines = lines.clone();
                let inverse_dir = match direction {
                    crate::edit::IndentDirection::In => crate::edit::IndentDirection::Out,
                    crate::edit::IndentDirection::Out => crate::edit::IndentDirection::In,
                };

                match direction {
                    crate::edit::IndentDirection::In => {
                        // Process lines in reverse order to preserve
                        // positions
                        let mut sorted = lines.clone();
                        sorted.sort_unstable();
                        for &line_idx in sorted.iter().rev() {
                            if line_idx >= self.rope.len_lines() {
                                continue;
                            }
                            let pos = Position::new(line_idx, 0);
                            let (evts, _inv) = self.apply_edit_inner(&EditCommand::Insert {
                                pos,
                                text: indent_str.to_string(),
                            })?;
                            all_events.extend(evts);
                        }
                    }
                    crate::edit::IndentDirection::Out => {
                        let mut sorted = lines.clone();
                        sorted.sort_unstable();
                        let mut actually_indented = Vec::new();
                        for &line_idx in sorted.iter().rev() {
                            if line_idx >= self.rope.len_lines() {
                                continue;
                            }
                            let line = self.rope.line(line_idx);
                            let spaces: usize =
                                line.chars().take(4).take_while(|c| *c == ' ').count();
                            if spaces > 0 {
                                let range = Range::new(
                                    Position::new(line_idx, 0),
                                    Position::new(line_idx, spaces),
                                );
                                let (evts, _inv) =
                                    self.apply_edit_inner(&EditCommand::Delete { range })?;
                                all_events.extend(evts);
                                actually_indented.push(line_idx);
                            }
                        }
                        inverse_lines = actually_indented;
                    }
                }

                let inverse = EditCommand::IndentLines {
                    lines: inverse_lines,
                    direction: inverse_dir,
                };
                Ok((all_events, inverse))
            }

            EditCommand::Batch(cmds) => {
                let mut all_events = Vec::new();
                let mut inverses = Vec::new();
                for cmd in cmds {
                    let (evts, inv) = self.apply_edit_inner(cmd)?;
                    all_events.extend(evts);
                    inverses.push(inv);
                }
                // Inverse of a batch is the batch of inverses in
                // reverse order.
                inverses.reverse();
                Ok((all_events, EditCommand::Batch(inverses)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_buffer() {
        let buf = Buffer::new(BufferId(1));
        assert_eq!(buf.id(), BufferId(1));
        assert!(buf.is_empty());
        assert!(!buf.is_dirty());
        assert!(buf.path().is_none());
        assert_eq!(buf.line_count(), 1); // empty rope has 1 line
        assert_eq!(buf.len_chars(), 0);
        assert_eq!(buf.len_bytes(), 0);
    }

    #[test]
    fn from_text_and_verify_content() {
        let buf = Buffer::from_text(BufferId(2), "hello\nworld");
        assert_eq!(buf.len_chars(), 11);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0).unwrap().to_string(), "hello\n");
        assert_eq!(buf.line(1).unwrap().to_string(), "world");
    }

    #[test]
    fn from_text_detects_line_ending() {
        let buf = Buffer::from_text(BufferId(3), "a\r\nb\r\nc\r\n");
        assert_eq!(buf.line_ending(), LineEnding::CrLf);
    }

    #[test]
    fn line_out_of_bounds_returns_none() {
        let buf = Buffer::from_text(BufferId(4), "abc");
        assert!(buf.line(5).is_none());
    }

    #[test]
    fn insert_text() {
        let mut buf = Buffer::from_text(BufferId(5), "ac");
        let events = buf
            .apply_edit(EditCommand::Insert {
                pos: Position::new(0, 1),
                text: "b".to_string(),
            })
            .unwrap();
        assert_eq!(buf.text().to_string(), "abc");
        assert_eq!(events.len(), 1);
        assert!(buf.is_dirty());
    }

    #[test]
    fn insert_multiline() {
        let mut buf = Buffer::from_text(BufferId(6), "ab");
        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 1),
            text: "x\ny".to_string(),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "ax\nyb");
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn delete_text() {
        let mut buf = Buffer::from_text(BufferId(7), "abcde");
        buf.apply_edit(EditCommand::Delete {
            range: Range::new(Position::new(0, 1), Position::new(0, 4)),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "ae");
    }

    #[test]
    fn replace_text() {
        let mut buf = Buffer::from_text(BufferId(8), "hello world");
        buf.apply_edit(EditCommand::Replace {
            range: Range::new(Position::new(0, 6), Position::new(0, 11)),
            text: "rust".to_string(),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "hello rust");
    }

    #[test]
    fn undo_insert() {
        let mut buf = Buffer::from_text(BufferId(9), "ab");
        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 1),
            text: "X".to_string(),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "aXb");

        let result = buf.undo().unwrap();
        assert!(result.is_some());
        assert_eq!(buf.text().to_string(), "ab");
    }

    #[test]
    fn undo_delete() {
        let mut buf = Buffer::from_text(BufferId(10), "abc");
        buf.apply_edit(EditCommand::Delete {
            range: Range::new(Position::new(0, 1), Position::new(0, 2)),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "ac");

        buf.undo().unwrap();
        assert_eq!(buf.text().to_string(), "abc");
    }

    #[test]
    fn undo_replace() {
        let mut buf = Buffer::from_text(BufferId(11), "abc");
        buf.apply_edit(EditCommand::Replace {
            range: Range::new(Position::new(0, 1), Position::new(0, 2)),
            text: "XY".to_string(),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "aXYc");

        buf.undo().unwrap();
        assert_eq!(buf.text().to_string(), "abc");
    }

    #[test]
    fn redo_after_undo() {
        let mut buf = Buffer::from_text(BufferId(12), "ab");
        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 1),
            text: "X".to_string(),
        })
        .unwrap();
        buf.undo().unwrap();
        assert_eq!(buf.text().to_string(), "ab");

        buf.redo().unwrap();
        assert_eq!(buf.text().to_string(), "aXb");
    }

    #[test]
    fn undo_on_empty_returns_none() {
        let mut buf = Buffer::new(BufferId(13));
        let result = buf.undo().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn redo_on_empty_returns_none() {
        let mut buf = Buffer::new(BufferId(14));
        let result = buf.redo().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn dirty_flag_after_edit_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let mut buf = Buffer::from_file(BufferId(15), &file_path).unwrap();
        assert!(!buf.is_dirty());

        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 5),
            text: "!".to_string(),
        })
        .unwrap();
        assert!(buf.is_dirty());

        buf.save().unwrap();
        assert!(!buf.is_dirty());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello!");
    }

    #[test]
    fn from_file_reads_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("sample.txt");
        std::fs::write(&file_path, "line1\nline2\n").unwrap();

        let buf = Buffer::from_file(BufferId(16), &file_path).unwrap();
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.path(), Some(file_path.as_path()));
    }

    #[test]
    fn from_file_not_found() {
        let result = Buffer::from_file(BufferId(17), Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn save_as_new_path() {
        let dir = tempfile::tempdir().unwrap();
        let mut buf = Buffer::from_text(BufferId(18), "content");
        let path = dir.path().join("out.txt");
        buf.save_as(&path).unwrap();
        assert_eq!(buf.path(), Some(path.as_path()));
        assert!(!buf.is_dirty());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn position_to_char_idx_bounds_checking() {
        let buf = Buffer::from_text(BufferId(19), "abc\ndef");
        // Valid positions
        assert!(buf.position_to_char_idx(Position::new(0, 0)).is_ok());
        assert!(buf.position_to_char_idx(Position::new(0, 3)).is_ok());
        assert!(buf.position_to_char_idx(Position::new(1, 0)).is_ok());
        assert!(buf.position_to_char_idx(Position::new(1, 3)).is_ok());

        // Out of bounds: line too large
        assert!(buf.position_to_char_idx(Position::new(5, 0)).is_err());
        // Out of bounds: col too large
        assert!(buf.position_to_char_idx(Position::new(1, 10)).is_err());
    }

    #[test]
    fn line_access() {
        let buf = Buffer::from_text(BufferId(20), "aaa\nbbb\nccc");
        assert_eq!(buf.line(0).unwrap().to_string(), "aaa\n");
        assert_eq!(buf.line(1).unwrap().to_string(), "bbb\n");
        assert_eq!(buf.line(2).unwrap().to_string(), "ccc");
        assert!(buf.line(3).is_none());
    }

    #[test]
    fn batch_edits() {
        let mut buf = Buffer::from_text(BufferId(21), "abc");
        let cmd = EditCommand::Batch(vec![
            EditCommand::Insert {
                pos: Position::new(0, 3),
                text: "d".to_string(),
            },
            EditCommand::Insert {
                pos: Position::new(0, 4),
                text: "e".to_string(),
            },
        ]);
        let events = buf.apply_edit(cmd).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(buf.text().to_string(), "abcde");
    }

    #[test]
    fn batch_undo() {
        let mut buf = Buffer::from_text(BufferId(22), "abc");
        let cmd = EditCommand::Batch(vec![
            EditCommand::Insert {
                pos: Position::new(0, 3),
                text: "d".to_string(),
            },
            EditCommand::Insert {
                pos: Position::new(0, 4),
                text: "e".to_string(),
            },
        ]);
        buf.apply_edit(cmd).unwrap();
        assert_eq!(buf.text().to_string(), "abcde");

        buf.undo().unwrap();
        assert_eq!(buf.text().to_string(), "abc");
    }

    #[test]
    fn buffer_id_next_is_unique() {
        let a = BufferId::next();
        let b = BufferId::next();
        assert_ne!(a, b);
    }

    #[test]
    fn edit_event_byte_offsets() {
        let mut buf = Buffer::from_text(BufferId(23), "abc");
        let events = buf
            .apply_edit(EditCommand::Insert {
                pos: Position::new(0, 1),
                text: "XY".to_string(),
            })
            .unwrap();
        let e = &events[0];
        assert_eq!(e.start_byte, 1);
        assert_eq!(e.old_end_byte, 1); // insert: old end == start
        assert_eq!(e.new_end_byte, 3); // 1 + 2 bytes for "XY"
    }

    #[test]
    fn insert_at_beginning() {
        let mut buf = Buffer::from_text(BufferId(24), "world");
        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 0),
            text: "hello ".to_string(),
        })
        .unwrap();
        assert_eq!(buf.text().to_string(), "hello world");
    }

    #[test]
    fn delete_entire_content() {
        let mut buf = Buffer::from_text(BufferId(25), "abc");
        buf.apply_edit(EditCommand::Delete {
            range: Range::new(Position::new(0, 0), Position::new(0, 3)),
        })
        .unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn insert_out_of_bounds_returns_error() {
        let mut buf = Buffer::from_text(BufferId(26), "ab");
        let result = buf.apply_edit(EditCommand::Insert {
            pos: Position::new(5, 0),
            text: "x".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn open_or_create_existing_file_reads_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("exists.txt");
        std::fs::write(&file_path, "existing content").unwrap();

        let buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        assert_eq!(buf.text().to_string(), "existing content");
        assert_eq!(buf.path(), Some(file_path.as_path()));
        assert!(!buf.is_dirty());
    }

    #[test]
    fn open_or_create_missing_file_creates_empty_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("does_not_exist.txt");

        // File does not exist yet
        assert!(!file_path.exists());

        let buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        assert!(buf.is_empty());
        assert_eq!(buf.path(), Some(file_path.as_path()));
        assert!(!buf.is_dirty());
    }

    #[test]
    fn open_or_create_missing_file_can_be_saved() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("new_file.txt");

        let mut buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        buf.apply_edit(EditCommand::Insert {
            pos: Position::new(0, 0),
            text: "hello new file".to_string(),
        })
        .unwrap();
        assert!(buf.is_dirty());

        buf.save().unwrap();
        assert!(!buf.is_dirty());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello new file");
    }

    #[test]
    fn open_or_create_missing_nested_parent_returns_error() {
        // When the parent directory doesn't exist, open_or_create
        // should still succeed (empty buffer with path set) — the
        // error will surface on save, not on open.
        let file_path = PathBuf::from("/tmp/smash_test_nonexistent_dir_12345/sub/file.txt");
        let buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        assert!(buf.is_empty());
        assert_eq!(buf.path(), Some(file_path.as_path()));
    }

    #[test]
    fn open_or_create_with_unicode_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("日本語ファイル.txt");

        let buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        assert!(buf.is_empty());
        assert_eq!(buf.path(), Some(file_path.as_path()));
    }

    #[test]
    fn open_or_create_with_spaces_in_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("my file with spaces.txt");
        std::fs::write(&file_path, "spaced content").unwrap();

        let buf = Buffer::open_or_create(BufferId::next(), &file_path).unwrap();
        assert_eq!(buf.text().to_string(), "spaced content");
    }
}
