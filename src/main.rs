use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use tracing::{error, info};

use smash_config::load_config;
use smash_core::buffer::{Buffer, BufferId};
use smash_core::edit::EditCommand;
use smash_core::message::MessageBuffer;
use smash_core::position::Position;
use smash_core::search::SearchQuery;
use smash_input::{
    create_default_keymap, create_vim_normal_layer, Command, KeyResolver, Keymap, ResolveResult,
};
use smash_platform::paths::DefaultPaths;
use smash_platform::paths::PlatformPaths;
use smash_platform::Platform;
use smash_syntax::{LanguageId, RegexHighlighter};
use smash_tui::{default_dark_theme, PaneTree, Rect, Renderer, TerminalBackend, Viewport};

/// The current input mode of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
enum InputMode {
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
}

/// Application state
#[allow(dead_code)]
struct App {
    buffer: Buffer,
    viewport: Viewport,
    renderer: Renderer,
    panes: PaneTree,
    resolver: KeyResolver,
    highlighter: Option<RegexHighlighter>,
    filename: Option<String>,
    messages: MessageBuffer,
    input_mode: InputMode,
    prompt_input: String,
    /// Secondary input for find-replace (replacement text).
    replace_input: String,
    /// Whether the replace prompt is focused (vs find prompt).
    replace_focused: bool,
    /// Fuzzy file finder.
    file_finder: Option<smash_core::fuzzy_finder::FileFinder>,
    /// Current finder results.
    finder_results: Vec<smash_core::fuzzy_finder::FileMatch>,
    running: bool,
}

impl App {
    fn new(width: u16, height: u16, file: Option<PathBuf>, keymap_preset: &str) -> Result<Self> {
        let id = BufferId::next();
        let (buffer, filename, highlighter) = match file {
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
                (buf, Some(name), hl)
            }
            None => (Buffer::new(id), None, None),
        };

        // Reserve 1 line for status bar
        let edit_height = height.saturating_sub(1);

        let default_layer = create_default_keymap();
        let mut keymap = Keymap::new(default_layer);

        // Apply keymap preset from config
        if keymap_preset == "vim" {
            keymap.push_layer(create_vim_normal_layer());
        }

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
        })
    }

    fn handle_command(&mut self, cmd: Command) {
        // When in a prompt mode, route input differently
        if self.input_mode != InputMode::Normal {
            self.handle_prompt_command(cmd);
            return;
        }

        match cmd {
            Command::Quit | Command::ForceQuit => {
                self.running = false;
            }
            Command::InsertChar(c) => {
                let pos = self.buffer.cursors().primary().position();
                let edit = EditCommand::Insert {
                    pos,
                    text: c.to_string(),
                };
                if self.buffer.apply_edit(edit).is_ok() {
                    let new_pos = if c == '\n' {
                        Position::new(pos.line + 1, 0)
                    } else {
                        Position::new(pos.line, pos.col + 1)
                    };
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(new_pos);
                }
            }
            Command::InsertNewline => {
                self.handle_command(Command::InsertChar('\n'));
            }
            Command::DeleteBackward => {
                let pos = self.buffer.cursors().primary().position();
                if pos.col > 0 {
                    let start = Position::new(pos.line, pos.col - 1);
                    let range = smash_core::position::Range::new(start, pos);
                    let edit = EditCommand::Delete { range };
                    if self.buffer.apply_edit(edit).is_ok() {
                        self.buffer.cursors_mut().primary_mut().set_position(start);
                    }
                } else if pos.line > 0 {
                    let prev_line = pos.line - 1;
                    let prev_len = self
                        .buffer
                        .line(prev_line)
                        .map(|l| l.len_chars().saturating_sub(1))
                        .unwrap_or(0);
                    let start = Position::new(prev_line, prev_len);
                    let range = smash_core::position::Range::new(start, pos);
                    let edit = EditCommand::Delete { range };
                    if self.buffer.apply_edit(edit).is_ok() {
                        self.buffer.cursors_mut().primary_mut().set_position(start);
                    }
                }
            }
            Command::DeleteForward => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                let end = if pos.col < line_len {
                    Position::new(pos.line, pos.col + 1)
                } else if pos.line + 1 < self.buffer.line_count() {
                    Position::new(pos.line + 1, 0)
                } else {
                    return;
                };
                let range = smash_core::position::Range::new(pos, end);
                let edit = EditCommand::Delete { range };
                let _ = self.buffer.apply_edit(edit);
            }
            Command::MoveLeft => {
                let pos = self.buffer.cursors().primary().position();
                if pos.col > 0 {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line, pos.col - 1));
                }
            }
            Command::MoveRight => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                if pos.col < line_len {
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(pos.line, pos.col + 1));
                }
            }
            Command::MoveUp => {
                let pos = self.buffer.cursors().primary().position();
                if pos.line > 0 {
                    let new_pos = self
                        .buffer
                        .clamp_position(Position::new(pos.line - 1, pos.col));
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(new_pos);
                }
            }
            Command::MoveDown => {
                let pos = self.buffer.cursors().primary().position();
                if pos.line + 1 < self.buffer.line_count() {
                    let new_pos = self
                        .buffer
                        .clamp_position(Position::new(pos.line + 1, pos.col));
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(new_pos);
                }
            }
            Command::MoveLineStart => {
                let pos = self.buffer.cursors().primary().position();
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line, 0));
            }
            Command::MoveLineEnd => {
                let pos = self.buffer.cursors().primary().position();
                let line_len = self
                    .buffer
                    .line(pos.line)
                    .map(|l| l.len_chars().saturating_sub(1))
                    .unwrap_or(0);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line, line_len));
            }
            Command::MoveBufferStart => {
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(0, 0));
            }
            Command::MoveBufferEnd => {
                let last = self.buffer.line_count().saturating_sub(1);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(last, 0));
            }
            Command::PageUp => {
                let lines = self.viewport.visible_lines();
                self.viewport.scroll_up(lines);
                let pos = self.buffer.cursors().primary().position();
                let new_line = pos.line.saturating_sub(lines);
                let new_pos = self.buffer.clamp_position(Position::new(new_line, pos.col));
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(new_pos);
            }
            Command::PageDown => {
                let lines = self.viewport.visible_lines();
                let total = self.buffer.line_count();
                self.viewport.scroll_down(lines, total);
                let pos = self.buffer.cursors().primary().position();
                let new_line = (pos.line + lines).min(total.saturating_sub(1));
                let new_pos = self.buffer.clamp_position(Position::new(new_line, pos.col));
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(new_pos);
            }
            Command::Undo => {
                let _ = self.buffer.undo();
            }
            Command::Redo => {
                let _ = self.buffer.redo();
            }
            Command::Save => {
                if self.buffer.path().is_some() {
                    match self.buffer.save() {
                        Ok(()) => {
                            self.messages.info("File saved");
                            info!("file saved");
                        }
                        Err(e) => {
                            self.messages.error(format!("Save failed: {}", e));
                            error!("save failed: {}", e);
                        }
                    }
                } else {
                    self.messages.warn("No file path set — use Save As");
                }
            }
            Command::Open => {
                self.input_mode = InputMode::PromptOpen;
                self.prompt_input.clear();
            }
            Command::Find => {
                self.input_mode = InputMode::PromptFind;
                self.prompt_input.clear();
            }
            Command::FindNext => {
                self.find_next();
            }
            Command::FindPrev => {
                self.find_prev();
            }
            Command::GoToLine => {
                self.input_mode = InputMode::PromptGoToLine;
                self.prompt_input.clear();
            }
            Command::FindReplace => {
                self.input_mode = InputMode::PromptFindReplace;
                self.prompt_input.clear();
                self.replace_input.clear();
                self.replace_focused = false;
            }
            Command::SaveAs => {
                self.input_mode = InputMode::PromptSaveAs;
                self.prompt_input.clear();
            }
            Command::Close => {
                // Close current buffer, quit if it was the last one
                self.running = false;
            }
            Command::OpenFileFinder => {
                self.input_mode = InputMode::FileFinder;
                self.prompt_input.clear();
                self.finder_results.clear();
                // Initialize the file finder if not already done
                if self.file_finder.is_none() {
                    if let Ok(cwd) = std::env::current_dir() {
                        let mut finder = smash_core::fuzzy_finder::FileFinder::new(cwd);
                        finder.index();
                        self.file_finder = Some(finder);
                    }
                }
            }
            Command::OpenCommandPalette => {
                self.messages.info("Command palette not yet implemented");
            }
            Command::MoveWordLeft => {
                self.move_word_left();
            }
            Command::MoveWordRight => {
                self.move_word_right();
            }
            Command::DeleteLine => {
                self.delete_current_line();
            }
            Command::SelectAll => {
                // Move cursor to buffer start; full selection tracking is TBD
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(0, 0));
                self.messages.info("SelectAll: cursor moved to start");
            }
            _ => {
                // Commands not yet implemented in prototype
            }
        }
    }

    /// Handle input while a prompt is active.
    fn handle_prompt_command(&mut self, cmd: Command) {
        match cmd {
            Command::InsertChar(c) => {
                match self.input_mode {
                    InputMode::PromptFindReplace if self.replace_focused => {
                        self.replace_input.push(c);
                    }
                    InputMode::FileFinder => {
                        self.prompt_input.push(c);
                        self.update_finder_results();
                    }
                    _ => {
                        self.prompt_input.push(c);
                    }
                }
                // Incremental search while typing in Find mode
                if self.input_mode == InputMode::PromptFind {
                    self.incremental_search();
                }
            }
            Command::InsertNewline => {
                // Confirm the prompt
                let input = self.prompt_input.clone();
                match self.input_mode {
                    InputMode::PromptOpen => self.confirm_open(&input),
                    InputMode::PromptFind => self.confirm_find(&input),
                    InputMode::PromptGoToLine => self.confirm_goto_line(&input),
                    InputMode::PromptSaveAs => self.confirm_save_as(&input),
                    InputMode::PromptFindReplace => {
                        if !self.replace_focused {
                            // Tab to replacement field
                            self.replace_focused = true;
                            return;
                        }
                        let replacement = self.replace_input.clone();
                        self.confirm_find_replace(&input, &replacement);
                    }
                    InputMode::FileFinder => {
                        self.confirm_file_finder();
                    }
                    InputMode::Normal => {}
                }
                if self.input_mode != InputMode::PromptFindReplace || !self.replace_focused {
                    self.input_mode = InputMode::Normal;
                    self.prompt_input.clear();
                    self.replace_input.clear();
                }
            }
            Command::DeleteBackward => {
                match self.input_mode {
                    InputMode::PromptFindReplace if self.replace_focused => {
                        self.replace_input.pop();
                    }
                    InputMode::FileFinder => {
                        self.prompt_input.pop();
                        self.update_finder_results();
                    }
                    _ => {
                        self.prompt_input.pop();
                    }
                }
                // Update incremental search
                if self.input_mode == InputMode::PromptFind {
                    self.incremental_search();
                }
            }
            Command::Quit | Command::ForceQuit => {
                // Treat as cancel
                self.input_mode = InputMode::Normal;
                self.prompt_input.clear();
                self.replace_input.clear();
                self.finder_results.clear();
            }
            _ => {
                // Ignore other commands while in prompt mode
            }
        }
    }

    /// Run incremental search on the current prompt input.
    fn incremental_search(&mut self) {
        let query_str = self.prompt_input.trim().to_string();
        if query_str.is_empty() {
            self.buffer.search_mut().clear();
            return;
        }
        let text = self.buffer.text().to_string();
        let search_query = SearchQuery::Plain {
            pattern: query_str,
            case_sensitive: false,
        };
        self.buffer.search_mut().set_query(search_query, &text);
    }

    /// Open a file (or create it) from the prompt.
    fn confirm_open(&mut self, filename: &str) {
        let filename = filename.trim();
        if filename.is_empty() {
            self.messages.warn("Open cancelled — no filename entered");
            return;
        }
        let path = std::path::PathBuf::from(filename);
        let id = BufferId::next();
        match Buffer::open_or_create(id, &path) {
            Ok(buf) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let lang = LanguageId::from_path(&path);
                self.highlighter = RegexHighlighter::new(lang).ok();
                self.buffer = buf;
                self.filename = Some(name.clone());
                if path.exists() {
                    self.messages.info(format!("Opened: {}", filename));
                } else {
                    self.messages.info(format!("New file: {}", filename));
                }
                info!("opened file: {}", filename);
            }
            Err(e) => {
                self.messages
                    .error(format!("Failed to open '{}': {}", filename, e));
                error!("open failed: {}", e);
            }
        }
    }

    /// Start a search from the prompt.
    fn confirm_find(&mut self, query: &str) {
        let query_str = query.trim();
        if query_str.is_empty() {
            self.buffer.search_mut().clear();
            self.messages.info("Search cleared");
            return;
        }
        let text = self.buffer.text().to_string();
        let search_query = SearchQuery::Plain {
            pattern: query_str.to_string(),
            case_sensitive: false,
        };
        self.buffer.search_mut().set_query(search_query, &text);
        let count = self.buffer.search().match_count();
        if count > 0 {
            self.messages
                .info(format!("Found {} match(es) for '{}'", count, query_str));
            // Jump to first match
            self.find_next();
        } else {
            self.messages
                .warn(format!("No matches found for '{}'", query_str));
        }
    }

    /// Jump to the next search match.
    fn find_next(&mut self) {
        if let Some(m) = self.buffer.search_mut().next_match() {
            let pos = m.range.start;
            self.buffer.cursors_mut().primary_mut().set_position(pos);
        } else {
            self.messages.info("No search results");
        }
    }

    /// Jump to the previous search match.
    fn find_prev(&mut self) {
        if let Some(m) = self.buffer.search_mut().prev_match() {
            let pos = m.range.start;
            self.buffer.cursors_mut().primary_mut().set_position(pos);
        } else {
            self.messages.info("No search results");
        }
    }

    /// Confirm go-to-line: jump cursor to a specific line number.
    fn confirm_goto_line(&mut self, input: &str) {
        let input = input.trim();
        if input.is_empty() {
            return;
        }
        match input.parse::<usize>() {
            Ok(0) => {
                self.messages.warn("Line numbers start at 1");
            }
            Ok(n) => {
                let target = (n - 1).min(self.buffer.line_count().saturating_sub(1));
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(target, 0));
                self.messages.info(format!("Jumped to line {}", target + 1));
            }
            Err(_) => {
                self.messages
                    .warn(format!("Invalid line number: '{}'", input));
            }
        }
    }

    /// Confirm save-as: save the buffer to a new file path.
    fn confirm_save_as(&mut self, input: &str) {
        let input = input.trim();
        if input.is_empty() {
            self.messages.warn("Save cancelled — no filename entered");
            return;
        }
        let path = std::path::PathBuf::from(input);
        match self.buffer.save_as(&path) {
            Ok(()) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                self.filename = Some(name);
                self.messages.info(format!("Saved as: {}", input));
                info!("saved as: {}", input);
            }
            Err(e) => {
                self.messages.error(format!("Save failed: {}", e));
                error!("save_as failed: {}", e);
            }
        }
    }

    /// Confirm find-replace: replace all occurrences.
    fn confirm_find_replace(&mut self, pattern: &str, replacement: &str) {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            self.messages.warn("Empty search pattern");
            self.input_mode = InputMode::Normal;
            self.prompt_input.clear();
            self.replace_input.clear();
            return;
        }
        let text = self.buffer.text().to_string();
        let new_text = text.replace(pattern, replacement);
        if new_text == text {
            self.messages
                .info(format!("No occurrences of '{}' found", pattern));
        } else {
            let count = text.matches(pattern).count();
            // Replace entire buffer content
            let full_range = smash_core::position::Range::new(
                Position::new(0, 0),
                Position::new(
                    self.buffer.line_count().saturating_sub(1),
                    self.buffer
                        .line(self.buffer.line_count().saturating_sub(1))
                        .map(|l| l.len_chars())
                        .unwrap_or(0),
                ),
            );
            let edit = EditCommand::Delete { range: full_range };
            let _ = self.buffer.apply_edit(edit);
            let edit = EditCommand::Insert {
                pos: Position::new(0, 0),
                text: new_text,
            };
            let _ = self.buffer.apply_edit(edit);
            self.messages
                .info(format!("Replaced {} occurrence(s)", count));
        }
        self.input_mode = InputMode::Normal;
        self.prompt_input.clear();
        self.replace_input.clear();
    }

    /// Move cursor one word to the left.
    fn move_word_left(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        if let Some(line_slice) = self.buffer.line(pos.line) {
            let line_str: String = line_slice.chars().collect();
            let chars: Vec<char> = line_str.chars().collect();
            let mut col = pos.col;
            // Skip whitespace backwards
            while col > 0 && chars.get(col - 1).is_some_and(|c| c.is_whitespace()) {
                col -= 1;
            }
            // Skip word chars backwards
            while col > 0
                && chars
                    .get(col - 1)
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                col -= 1;
            }
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(pos.line, col));
        } else if pos.line > 0 {
            // Move to end of previous line
            let prev_line = pos.line - 1;
            let prev_len = self
                .buffer
                .line(prev_line)
                .map(|l| l.len_chars().saturating_sub(1))
                .unwrap_or(0);
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(prev_line, prev_len));
        }
    }

    /// Move cursor one word to the right.
    fn move_word_right(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        if let Some(line_slice) = self.buffer.line(pos.line) {
            let line_str: String = line_slice.chars().collect();
            let chars: Vec<char> = line_str.chars().collect();
            let len = chars.len().saturating_sub(1); // exclude newline
            let mut col = pos.col;
            // Skip word chars forward
            while col < len && (chars[col].is_alphanumeric() || chars[col] == '_') {
                col += 1;
            }
            // Skip whitespace forward
            while col < len && chars[col].is_whitespace() {
                col += 1;
            }
            if col != pos.col {
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line, col));
            } else if pos.line + 1 < self.buffer.line_count() {
                // Move to start of next line
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(pos.line + 1, 0));
            }
        }
    }

    /// Delete the entire current line.
    fn delete_current_line(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        let line_count = self.buffer.line_count();
        if line_count == 0 {
            return;
        }
        let start = Position::new(pos.line, 0);
        let end = if pos.line + 1 < line_count {
            Position::new(pos.line + 1, 0)
        } else if pos.line > 0 {
            // Last line: delete from end of previous line
            let prev_len = self
                .buffer
                .line(pos.line - 1)
                .map(|l| l.len_chars().saturating_sub(1))
                .unwrap_or(0);
            let actual_start = Position::new(pos.line - 1, prev_len);
            let actual_end = Position::new(
                pos.line,
                self.buffer
                    .line(pos.line)
                    .map(|l| l.len_chars())
                    .unwrap_or(0),
            );
            let range = smash_core::position::Range::new(actual_start, actual_end);
            let edit = EditCommand::Delete { range };
            if self.buffer.apply_edit(edit).is_ok() {
                let new_line = pos.line.saturating_sub(1);
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(new_line, 0));
            }
            return;
        } else {
            // Single-line buffer: clear the line
            Position::new(0, self.buffer.line(0).map(|l| l.len_chars()).unwrap_or(0))
        };
        let range = smash_core::position::Range::new(start, end);
        let edit = EditCommand::Delete { range };
        if self.buffer.apply_edit(edit).is_ok() {
            let new_line = pos.line.min(self.buffer.line_count().saturating_sub(1));
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(new_line, 0));
        }
    }

    /// Update the fuzzy finder results from prompt input.
    fn update_finder_results(&mut self) {
        if let Some(ref finder) = self.file_finder {
            self.finder_results = finder.search(&self.prompt_input, 20);
        }
    }

    /// Confirm file finder selection: open the first result.
    fn confirm_file_finder(&mut self) {
        if let Some(first) = self.finder_results.first() {
            let path = first.path().to_path_buf();
            self.input_mode = InputMode::Normal;
            self.prompt_input.clear();
            self.finder_results.clear();
            self.confirm_open(&path.to_string_lossy());
        } else {
            self.messages.info("No matching files");
            self.input_mode = InputMode::Normal;
            self.prompt_input.clear();
        }
    }

    fn render(&mut self, backend: &mut dyn TerminalBackend) -> Result<()> {
        let (w, h) = backend.size()?;

        let pos = self.buffer.cursors().primary().position();
        self.viewport.scroll_to_cursor(pos.line, pos.col);

        let status_h = 1u16;
        let edit_area = Rect::new(0, 0, w, h.saturating_sub(status_h));
        let status_area = Rect::new(0, h.saturating_sub(status_h), w, status_h);

        let theme = default_dark_theme();

        self.renderer.render_buffer(
            &self.buffer,
            &self.viewport,
            edit_area,
            &theme,
            self.highlighter
                .as_ref()
                .map(|h| h as &dyn smash_syntax::HighlightEngine),
            true,
        );

        // Determine status bar content based on mode
        match &self.input_mode {
            InputMode::PromptOpen => {
                let prompt_text = format!("Open file: {}", self.prompt_input);
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::PromptFind => {
                let match_count = self.buffer.search().match_count();
                let prompt_text = if match_count > 0 {
                    format!("Find: {} ({} matches)", self.prompt_input, match_count)
                } else {
                    format!("Find: {}", self.prompt_input)
                };
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::PromptGoToLine => {
                let prompt_text = format!("Go to line: {}", self.prompt_input);
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::PromptSaveAs => {
                let prompt_text = format!("Save as: {}", self.prompt_input);
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::PromptFindReplace => {
                let prompt_text = if self.replace_focused {
                    format!(
                        "Replace '{}' with: {}",
                        self.prompt_input, self.replace_input
                    )
                } else {
                    format!("Find (for replace): {}", self.prompt_input)
                };
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::FileFinder => {
                let result_count = self.finder_results.len();
                let prompt_text = if result_count > 0 {
                    let first = self.finder_results[0].relative_path();
                    format!(
                        "Find file: {} ({} results, top: {})",
                        self.prompt_input, result_count, first
                    )
                } else {
                    format!("Find file: {}", self.prompt_input)
                };
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
            InputMode::Normal => {
                // Show last message if recent, otherwise normal status
                let status_text = if let Some(msg) = self.messages.last() {
                    format!(
                        "{} | {}",
                        self.filename.as_deref().unwrap_or("[scratch]"),
                        msg.text()
                    )
                } else {
                    self.filename.as_deref().unwrap_or("[scratch]").to_string()
                };
                self.renderer.render_status_bar(
                    status_area,
                    &status_text,
                    pos.line,
                    pos.col,
                    self.buffer.is_dirty(),
                    &theme,
                );
            }
        }

        self.renderer.flush_to_backend(backend)?;

        let gutter_w = 5u16;
        let screen_col = gutter_w + (pos.col.saturating_sub(self.viewport.left_col())) as u16;
        let screen_row = (pos.line.saturating_sub(self.viewport.top_line())) as u16;
        backend.move_cursor(screen_col, screen_row)?;
        backend.show_cursor()?;

        Ok(())
    }
}

fn run_editor(file: Option<PathBuf>) -> Result<()> {
    let paths = DefaultPaths::new().context("failed to detect platform paths")?;

    // Direct tracing output to a log file so it never bleeds into the TUI.
    let log_dir = paths.log_dir();
    std::fs::create_dir_all(&log_dir).ok();
    let log_file = std::fs::File::create(log_dir.join("smash.log"))
        .unwrap_or_else(|_| std::fs::File::create("/dev/null").expect("cannot open /dev/null"));
    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false)
        .init();

    let config_dir = paths.config_dir();
    let project_dir = std::env::current_dir().ok();
    let config = load_config(&config_dir, project_dir.as_deref()).unwrap_or_else(|e| {
        error!("config load failed, using defaults: {}", e);
        smash_config::Config::default()
    });

    let _platform = Platform::default_platform().context("failed to initialize platform")?;

    let (width, height) = crossterm::terminal::size().context("failed to get terminal size")?;

    let mut app = App::new(width, height, file, &config.keymap.preset)?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let mut backend = CrosstermBackend::new();

    if let Err(e) = app.render(&mut backend) {
        error!("render error: {}", e);
    }

    while app.running {
        if event::poll(Duration::from_millis(100))? {
            let raw_event = event::read()?;

            if let Event::Resize(w, h) = raw_event {
                app.viewport
                    .resize(h.saturating_sub(1) as usize, w as usize);
                app.renderer.resize(w, h);
                if let Err(e) = app.render(&mut backend) {
                    error!("render error: {}", e);
                }
                continue;
            }

            if let Some(input) = smash_input::event::from_crossterm(raw_event) {
                // Handle Esc to cancel prompts
                if let smash_input::InputEvent::Key(ke) = &input {
                    if ke.key == smash_input::Key::Esc && app.input_mode != InputMode::Normal {
                        app.input_mode = InputMode::Normal;
                        app.prompt_input.clear();
                        if let Err(e) = app.render(&mut backend) {
                            error!("render error: {}", e);
                        }
                        continue;
                    }
                }

                match app.resolver.resolve(input) {
                    ResolveResult::Command(cmd) => {
                        app.handle_command(cmd);
                    }
                    ResolveResult::WaitingForMore => {
                        continue;
                    }
                    ResolveResult::Unresolved => {}
                }
            }

            if let Err(e) = app.render(&mut backend) {
                error!("render error: {}", e);
            }
        }
    }

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;

    info!("smash exited cleanly");
    Ok(())
}

/// Minimal crossterm backend for production use
struct CrosstermBackend {
    stdout: std::io::Stdout,
}

impl CrosstermBackend {
    fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
        }
    }
}

impl TerminalBackend for CrosstermBackend {
    fn size(&self) -> Result<(u16, u16), smash_tui::TuiError> {
        crossterm::terminal::size().map_err(smash_tui::TuiError::Io)
    }

    fn move_cursor(&mut self, col: u16, row: u16) -> Result<(), smash_tui::TuiError> {
        use crossterm::cursor::MoveTo;
        crossterm::execute!(self.stdout, MoveTo(col, row)).map_err(smash_tui::TuiError::Io)
    }

    fn show_cursor(&mut self) -> Result<(), smash_tui::TuiError> {
        crossterm::execute!(self.stdout, crossterm::cursor::Show).map_err(smash_tui::TuiError::Io)
    }

    fn hide_cursor(&mut self) -> Result<(), smash_tui::TuiError> {
        crossterm::execute!(self.stdout, crossterm::cursor::Hide).map_err(smash_tui::TuiError::Io)
    }

    fn clear(&mut self) -> Result<(), smash_tui::TuiError> {
        use crossterm::terminal::{Clear, ClearType};
        crossterm::execute!(self.stdout, Clear(ClearType::All)).map_err(smash_tui::TuiError::Io)
    }

    fn write_cell(
        &mut self,
        col: u16,
        row: u16,
        cell: &smash_tui::Cell,
    ) -> Result<(), smash_tui::TuiError> {
        use crossterm::cursor::MoveTo;
        use crossterm::style::{Print, SetBackgroundColor, SetForegroundColor};
        let fg = to_crossterm_color(cell.style.fg);
        let bg = to_crossterm_color(cell.style.bg);
        crossterm::execute!(
            self.stdout,
            MoveTo(col, row),
            SetForegroundColor(fg),
            SetBackgroundColor(bg),
            Print(cell.ch)
        )
        .map_err(smash_tui::TuiError::Io)
    }

    fn flush(&mut self) -> Result<(), smash_tui::TuiError> {
        use std::io::Write;
        self.stdout.flush().map_err(smash_tui::TuiError::Io)
    }

    fn enter_alternate_screen(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<(), smash_tui::TuiError> {
        Ok(())
    }
}

fn to_crossterm_color(color: smash_tui::Color) -> crossterm::style::Color {
    match color {
        smash_tui::Color::Reset => crossterm::style::Color::Reset,
        smash_tui::Color::Black => crossterm::style::Color::Black,
        smash_tui::Color::Red => crossterm::style::Color::DarkRed,
        smash_tui::Color::Green => crossterm::style::Color::DarkGreen,
        smash_tui::Color::Yellow => crossterm::style::Color::DarkYellow,
        smash_tui::Color::Blue => crossterm::style::Color::DarkBlue,
        smash_tui::Color::Magenta => crossterm::style::Color::DarkMagenta,
        smash_tui::Color::Cyan => crossterm::style::Color::DarkCyan,
        smash_tui::Color::White => crossterm::style::Color::White,
        smash_tui::Color::Rgb(r, g, b) => crossterm::style::Color::Rgb { r, g, b },
        smash_tui::Color::Indexed(i) => crossterm::style::Color::AnsiValue(i),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = args.get(1).map(PathBuf::from);

    if let Err(e) = run_editor(file) {
        eprintln!("smash: {:#}", e);
        std::process::exit(1);
    }
}
