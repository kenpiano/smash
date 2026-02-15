mod commands;
mod lsp;
mod render;

use anyhow::{Context, Result};
use std::path::PathBuf;

use smash_core::buffer::{Buffer, BufferId};
use smash_core::message::MessageBuffer;
use smash_core::position::Position;
use smash_input::{create_default_keymap, create_emacs_keymap, KeyResolver, Keymap};
use smash_lsp::{CompletionItem, Diagnostic};
use smash_syntax::{LanguageId, RegexHighlighter};
use smash_tui::{PaneTree, Renderer, Viewport};

use crate::lsp_types::{LspCommand, LspEvent};

/// Maximum number of entries in the jump stack.
const JUMP_STACK_MAX: usize = 100;

/// A saved cursor location for jump-back / jump-forward navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JumpLocation {
    /// Absolute file path (or None for an unnamed buffer).
    pub(crate) path: Option<PathBuf>,
    /// Cursor position at the time of the jump.
    pub(crate) position: Position,
}

impl JumpLocation {
    pub(crate) fn new(path: Option<PathBuf>, position: Position) -> Self {
        Self { path, position }
    }
}

/// A stack that records jump locations so the user can navigate back/forward.
#[derive(Debug)]
pub(crate) struct JumpStack {
    /// Past locations (the "back" stack).
    entries: Vec<JumpLocation>,
    /// Locations we left via jump-back (the "forward" stack).
    forward: Vec<JumpLocation>,
}

impl JumpStack {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            forward: Vec::new(),
        }
    }

    /// Push a new location onto the back stack and clear forward history.
    pub(crate) fn push(&mut self, loc: JumpLocation) {
        // De-duplicate: skip if the top of the stack is identical.
        if self.entries.last() == Some(&loc) {
            return;
        }
        self.entries.push(loc);
        if self.entries.len() > JUMP_STACK_MAX {
            self.entries.remove(0);
        }
        self.forward.clear();
    }

    /// Pop the most recent location (for jump-back).
    /// `current` is the location the user is currently at; it is pushed
    /// onto the forward stack so jump-forward can restore it.
    pub(crate) fn pop_back(&mut self, current: JumpLocation) -> Option<JumpLocation> {
        let loc = self.entries.pop()?;
        self.forward.push(current);
        Some(loc)
    }

    /// Pop from the forward stack (for jump-forward).
    /// `current` is pushed back onto the back stack.
    pub(crate) fn pop_forward(&mut self, current: JumpLocation) -> Option<JumpLocation> {
        let loc = self.forward.pop()?;
        self.entries.push(current);
        Some(loc)
    }

    /// Number of back entries.
    #[cfg(test)]
    pub(crate) fn back_len(&self) -> usize {
        self.entries.len()
    }

    /// Number of forward entries.
    #[cfg(test)]
    pub(crate) fn forward_len(&self) -> usize {
        self.forward.len()
    }
}

/// Return the "content length" of a rope line slice.
///
/// Ropey includes a trailing `\n` in `len_chars()` for every line except
/// (potentially) the last one.  This helper subtracts 1 only when the line
/// actually ends with a newline so that the cursor can reach the true end
/// of text on the final line.
pub(crate) fn line_content_len(line: smash_core::buffer::RopeSlice<'_>) -> usize {
    let len = line.len_chars();
    if len > 0 && line.char(len - 1) == '\n' {
        len - 1
    } else {
        len
    }
}

/// The current input mode of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputMode {
    /// Normal editing mode.
    Normal,
    /// Prompt for a filename to open.
    PromptOpen,
    /// Prompt for a search query.
    PromptFind,
    /// Prompt for a line number (Ctrl+G).
    PromptGoToLine,
    /// Prompt for find-and-replace.
    PromptFindReplace,
    /// Prompt for Save-As filename.
    PromptSaveAs,
    /// Fuzzy file finder overlay.
    FileFinder,
    /// Prompt for LSP rename (new symbol name).
    PromptLspRename,
}

/// Application state
#[allow(dead_code)]
pub(crate) struct App {
    pub(crate) buffer: Buffer,
    pub(crate) viewport: Viewport,
    pub(crate) renderer: Renderer,
    pub(crate) panes: PaneTree,
    pub(crate) resolver: KeyResolver,
    pub(crate) highlighter: Option<RegexHighlighter>,
    pub(crate) filename: Option<String>,
    pub(crate) messages: MessageBuffer,
    pub(crate) input_mode: InputMode,
    pub(crate) prompt_input: String,
    /// Secondary input for find-replace (replacement text).
    pub(crate) replace_input: String,
    /// Whether the replace prompt is focused (vs find prompt).
    pub(crate) replace_focused: bool,
    /// Fuzzy file finder.
    pub(crate) file_finder: Option<smash_core::fuzzy_finder::FileFinder>,
    /// Current finder results.
    pub(crate) finder_results: Vec<smash_core::fuzzy_finder::FileMatch>,
    pub(crate) running: bool,
    // --- LSP integration ---
    /// Channel to send commands to the LSP async task.
    pub(crate) lsp_cmd_tx: tokio::sync::mpsc::Sender<LspCommand>,
    /// Channel to receive events from the LSP async task.
    pub(crate) lsp_evt_rx: std::sync::mpsc::Receiver<LspEvent>,
    /// Current document version (incremented on each edit for didChange).
    pub(crate) document_version: i32,
    /// Current language ID for the active buffer.
    pub(crate) language_id: Option<String>,
    /// Whether LSP is enabled in config.
    pub(crate) lsp_enabled: bool,
    /// LSP server configs from config file.
    pub(crate) lsp_server_configs: std::collections::HashMap<String, smash_config::LspServerEntry>,
    /// Whether an LSP server has been started for the current language.
    pub(crate) lsp_server_started: bool,
    /// Diagnostics for the current file.
    pub(crate) current_diagnostics: Vec<Diagnostic>,
    /// Current diagnostic index for next/prev navigation.
    pub(crate) diagnostic_index: usize,
    /// Last hover text to display.
    pub(crate) hover_text: Option<String>,
    /// Completion items from LSP.
    pub(crate) completion_items: Vec<CompletionItem>,
    /// Selected completion index.
    pub(crate) completion_index: usize,
    /// Whether to normalize macOS Option key to Alt.
    pub(crate) option_as_alt: bool,
    // --- Jump navigation ---
    /// Stack for jump-back / jump-forward navigation across files.
    pub(crate) jump_stack: JumpStack,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        width: u16,
        height: u16,
        file: Option<PathBuf>,
        keymap_preset: &str,
        lsp_cmd_tx: tokio::sync::mpsc::Sender<LspCommand>,
        lsp_evt_rx: std::sync::mpsc::Receiver<LspEvent>,
        lsp_enabled: bool,
        lsp_server_configs: std::collections::HashMap<String, smash_config::LspServerEntry>,
        option_as_alt: bool,
    ) -> Result<Self> {
        let id = BufferId::next();
        let (buffer, filename, highlighter, lang_id) = match file {
            Some(ref path) => {
                let buf = Buffer::open_or_create(id, path)
                    .with_context(|| format!("failed to open: {}", path.display()))?;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let lang = LanguageId::from_path(path);
                let hl = RegexHighlighter::new(lang).ok();
                let lang_str = lang.as_str().to_string();
                (buf, Some(name), hl, Some(lang_str))
            }
            None => (Buffer::new(id), None, None, None),
        };

        // Reserve 1 line for status bar
        let edit_height = height.saturating_sub(1);

        // Select base keymap from config preset
        let base_layer = if keymap_preset == "emacs" {
            create_emacs_keymap()
        } else {
            create_default_keymap()
        };
        let keymap = Keymap::new(base_layer);

        let resolver = KeyResolver::new(keymap);

        Ok(Self {
            buffer,
            viewport: Viewport::new(edit_height as usize, width as usize),
            renderer: Renderer::new(width, height),
            panes: PaneTree::new(),
            resolver,
            highlighter,
            filename,
            messages: MessageBuffer::new(),
            input_mode: InputMode::Normal,
            prompt_input: String::new(),
            replace_input: String::new(),
            replace_focused: false,
            file_finder: None,
            finder_results: Vec::new(),
            running: true,
            lsp_cmd_tx,
            lsp_evt_rx,
            document_version: 1,
            language_id: lang_id,
            lsp_enabled,
            lsp_server_configs,
            lsp_server_started: false,
            current_diagnostics: Vec::new(),
            diagnostic_index: 0,
            hover_text: None,
            completion_items: Vec::new(),
            completion_index: 0,
            option_as_alt,
            jump_stack: JumpStack::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smash_core::buffer::{Buffer, BufferId};
    use smash_input::Command;

    /// Create a test App instance with dummy LSP channels.
    fn test_app() -> App {
        let (lsp_cmd_tx, _lsp_cmd_rx) = tokio::sync::mpsc::channel(1);
        let (_lsp_evt_tx, lsp_evt_rx) = std::sync::mpsc::channel();
        App::new(
            80,
            24,
            None,
            "default",
            lsp_cmd_tx,
            lsp_evt_rx,
            false,
            std::collections::HashMap::new(),
            false,
        )
        .unwrap()
    }

    #[test]
    fn path_to_uri_absolute_path() {
        let uri = App::path_to_uri(std::path::Path::new("/home/user/project/main.rs"));
        assert_eq!(uri, "file:///home/user/project/main.rs");
    }

    #[test]
    fn path_to_uri_relative_path_becomes_absolute() {
        let uri = App::path_to_uri(std::path::Path::new("hello.c"));
        assert!(
            uri.starts_with("file:///"),
            "relative path must produce absolute file URI, got: {}",
            uri
        );
        assert!(
            uri.ends_with("/hello.c"),
            "URI must end with the filename, got: {}",
            uri
        );
    }

    #[test]
    fn current_uri_returns_none_for_empty_buffer() {
        let app = test_app();
        assert!(app.current_uri().is_none());
    }

    #[test]
    fn line_content_len_excludes_trailing_newline() {
        let buf = Buffer::from_text(BufferId::next(), "hello\nworld\n");
        // First line "hello\n" → content length 5
        let line0 = buf.line(0).unwrap();
        assert_eq!(line_content_len(line0), 5);
        // Second line "world\n" → content length 5
        let line1 = buf.line(1).unwrap();
        assert_eq!(line_content_len(line1), 5);
    }

    #[test]
    fn line_content_len_last_line_no_newline() {
        let buf = Buffer::from_text(BufferId::next(), "hello\nworld");
        // First line "hello\n" → content length 5
        let line0 = buf.line(0).unwrap();
        assert_eq!(line_content_len(line0), 5);
        // Last line "world" (no newline) → content length 5
        let line1 = buf.line(1).unwrap();
        assert_eq!(line_content_len(line1), 5);
    }

    #[test]
    fn line_content_len_single_line_no_newline() {
        let buf = Buffer::from_text(BufferId::next(), "abc");
        let line = buf.line(0).unwrap();
        // "abc" has no trailing newline → content length 3
        assert_eq!(line_content_len(line), 3);
    }

    #[test]
    fn line_content_len_empty_buffer() {
        let buf = Buffer::new(BufferId::next());
        let line = buf.line(0).unwrap();
        assert_eq!(line_content_len(line), 0);
    }

    #[test]
    fn cursor_can_reach_end_of_last_line_without_newline() {
        // Simulate: buffer contains "abc" (no trailing newline).
        // The cursor should be able to move to column 3 (after 'c').
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "abc");

        // MoveLineEnd should place cursor at column 3
        app.handle_command(Command::MoveLineEnd);
        let pos = app.buffer.cursors().primary().position();
        assert_eq!(pos.col, 3, "cursor should be at col 3 (after 'c')");

        // Inserting a char at end-of-content should work
        app.handle_command(Command::InsertChar('d'));
        assert_eq!(app.buffer.text().to_string(), "abcd");
    }

    #[test]
    fn cursor_can_insert_at_eof_after_newline() {
        // Buffer: "abc\n" — last line is empty.
        // Cursor should be able to type on the empty last line.
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "abc\n");

        // Move to buffer end (empty last line)
        app.handle_command(Command::MoveBufferEnd);
        app.handle_command(Command::InsertChar('x'));
        let text = app.buffer.text().to_string();
        assert!(
            text.contains("x"),
            "should be able to insert on empty last line"
        );
    }

    #[test]
    fn delete_backward_at_end_of_last_line() {
        // Buffer: "abc" — cursor at col 3 (after 'c').
        // DeleteBackward should delete 'c'.
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "abc");

        // Move to end
        app.handle_command(Command::MoveLineEnd);
        let pos = app.buffer.cursors().primary().position();
        assert_eq!(pos.col, 3);

        // Delete backward
        app.handle_command(Command::DeleteBackward);
        assert_eq!(app.buffer.text().to_string(), "ab");
    }

    // =====================================================================
    // JumpStack unit tests
    // =====================================================================

    #[test]
    fn jump_stack_new_is_empty() {
        let stack = JumpStack::new();
        assert_eq!(stack.back_len(), 0);
        assert_eq!(stack.forward_len(), 0);
    }

    #[test]
    fn jump_stack_push_increases_back_len() {
        let mut stack = JumpStack::new();
        stack.push(JumpLocation::new(None, Position::new(0, 0)));
        assert_eq!(stack.back_len(), 1);
        stack.push(JumpLocation::new(None, Position::new(5, 3)));
        assert_eq!(stack.back_len(), 2);
    }

    #[test]
    fn jump_stack_push_clears_forward() {
        let mut stack = JumpStack::new();
        stack.push(JumpLocation::new(None, Position::new(0, 0)));
        stack.push(JumpLocation::new(None, Position::new(5, 0)));
        // pop_back puts current on forward
        let current = JumpLocation::new(None, Position::new(10, 0));
        stack.pop_back(current);
        assert_eq!(stack.forward_len(), 1);
        // push clears forward
        stack.push(JumpLocation::new(None, Position::new(20, 0)));
        assert_eq!(stack.forward_len(), 0);
    }

    #[test]
    fn jump_stack_pop_back_returns_last_pushed() {
        let mut stack = JumpStack::new();
        let loc_a = JumpLocation::new(None, Position::new(1, 0));
        let loc_b = JumpLocation::new(None, Position::new(2, 0));
        stack.push(loc_a.clone());
        stack.push(loc_b.clone());

        let current = JumpLocation::new(None, Position::new(3, 0));
        let popped = stack.pop_back(current);
        assert_eq!(popped, Some(loc_b));
    }

    #[test]
    fn jump_stack_pop_back_moves_current_to_forward() {
        let mut stack = JumpStack::new();
        stack.push(JumpLocation::new(None, Position::new(1, 0)));

        let current = JumpLocation::new(None, Position::new(5, 0));
        stack.pop_back(current.clone());
        assert_eq!(stack.forward_len(), 1);

        // pop_forward should give us back the current we just passed in
        let dummy = JumpLocation::new(None, Position::new(1, 0));
        let fwd = stack.pop_forward(dummy);
        assert_eq!(fwd, Some(current));
    }

    #[test]
    fn jump_stack_pop_back_empty_returns_none() {
        let mut stack = JumpStack::new();
        let current = JumpLocation::new(None, Position::new(0, 0));
        assert!(stack.pop_back(current).is_none());
    }

    #[test]
    fn jump_stack_pop_forward_empty_returns_none() {
        let mut stack = JumpStack::new();
        let current = JumpLocation::new(None, Position::new(0, 0));
        assert!(stack.pop_forward(current).is_none());
    }

    #[test]
    fn jump_stack_round_trip_back_and_forward() {
        let mut stack = JumpStack::new();
        let loc_a = JumpLocation::new(None, Position::new(1, 0));
        let loc_b = JumpLocation::new(None, Position::new(2, 0));
        let loc_c = JumpLocation::new(None, Position::new(3, 0));
        stack.push(loc_a.clone());
        stack.push(loc_b.clone());

        // Go back from C → B
        let back1 = stack.pop_back(loc_c.clone()).unwrap();
        assert_eq!(back1, loc_b);

        // Go back from B → A
        let back2 = stack.pop_back(loc_b.clone()).unwrap();
        assert_eq!(back2, loc_a);

        // Go forward from A → B
        let fwd1 = stack.pop_forward(loc_a.clone()).unwrap();
        assert_eq!(fwd1, loc_b);

        // Go forward from B → C
        let fwd2 = stack.pop_forward(loc_b.clone()).unwrap();
        assert_eq!(fwd2, loc_c);
    }

    #[test]
    fn jump_stack_deduplicates_consecutive_same_location() {
        let mut stack = JumpStack::new();
        let loc = JumpLocation::new(None, Position::new(5, 3));
        stack.push(loc.clone());
        stack.push(loc.clone());
        stack.push(loc.clone());
        assert_eq!(stack.back_len(), 1, "duplicate pushes should be ignored");
    }

    #[test]
    fn jump_stack_respects_max_size() {
        let mut stack = JumpStack::new();
        for i in 0..(JUMP_STACK_MAX + 20) {
            stack.push(JumpLocation::new(None, Position::new(i, 0)));
        }
        assert_eq!(stack.back_len(), JUMP_STACK_MAX);
    }

    #[test]
    fn jump_location_with_path() {
        let loc = JumpLocation::new(Some(PathBuf::from("/tmp/foo.rs")), Position::new(10, 5));
        assert_eq!(loc.path, Some(PathBuf::from("/tmp/foo.rs")));
        assert_eq!(loc.position, Position::new(10, 5));
    }

    // =====================================================================
    // App-level jump navigation tests
    // =====================================================================

    #[test]
    fn app_jump_back_with_empty_stack_shows_message() {
        let mut app = test_app();
        app.handle_command(Command::JumpBack);
        // Should not panic, "No previous location" message displayed
        assert_eq!(app.jump_stack.back_len(), 0);
    }

    #[test]
    fn app_jump_forward_with_empty_stack_shows_message() {
        let mut app = test_app();
        app.handle_command(Command::JumpForward);
        // Should not panic, "No next location" message displayed
        assert_eq!(app.jump_stack.forward_len(), 0);
    }

    #[test]
    fn app_push_jump_records_current_position() {
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "line 0\nline 1\nline 2\n");

        // Move cursor to line 1
        app.handle_command(Command::MoveDown);
        app.push_jump();

        assert_eq!(app.jump_stack.back_len(), 1);
    }

    #[test]
    fn app_jump_back_restores_position() {
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "line 0\nline 1\nline 2\n");

        // Record position at line 0
        app.push_jump();
        let original_pos = app.buffer.cursors().primary().position();

        // Move cursor to line 2
        app.handle_command(Command::MoveDown);
        app.handle_command(Command::MoveDown);

        // Jump back should restore to line 0
        app.handle_command(Command::JumpBack);
        let restored_pos = app.buffer.cursors().primary().position();
        assert_eq!(restored_pos, original_pos);
    }

    #[test]
    fn app_jump_back_then_forward_round_trip() {
        let mut app = test_app();
        let id = BufferId::next();
        app.buffer = Buffer::from_text(id, "line 0\nline 1\nline 2\n");

        // Push position at line 0
        app.push_jump();

        // Move to line 2
        app.handle_command(Command::MoveDown);
        app.handle_command(Command::MoveDown);
        let line2_pos = app.buffer.cursors().primary().position();

        // Jump back (saves line 2 to forward, restores line 0)
        app.handle_command(Command::JumpBack);
        assert_eq!(
            app.buffer.cursors().primary().position(),
            Position::new(0, 0)
        );

        // Jump forward (saves line 0 to back, restores line 2)
        app.handle_command(Command::JumpForward);
        assert_eq!(app.buffer.cursors().primary().position(), line2_pos);
    }
}
