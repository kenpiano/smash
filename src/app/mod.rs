mod commands;
mod lsp;
mod render;

use anyhow::{Context, Result};
use std::path::PathBuf;

use smash_core::buffer::{Buffer, BufferId};
use smash_core::message::MessageBuffer;
use smash_input::{create_default_keymap, create_emacs_keymap, KeyResolver, Keymap};
use smash_lsp::{CompletionItem, Diagnostic};
use smash_syntax::{LanguageId, RegexHighlighter};
use smash_tui::{PaneTree, Renderer, Viewport};

use crate::lsp_types::{LspCommand, LspEvent};

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
}
