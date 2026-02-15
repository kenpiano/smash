use smash_core::buffer::BufferId;
use smash_core::edit::EditCommand;
use smash_core::position::Position;
use smash_core::search::SearchQuery;
use smash_input::Command;
use smash_syntax::{LanguageId, RegexHighlighter};
use tracing::{error, info};

use super::{line_content_len, App, InputMode, JumpLocation};

// =========================================================================
// Command dispatch
// =========================================================================

impl App {
    /// Top-level command handler (Normal mode).
    pub(crate) fn handle_command(&mut self, cmd: Command) {
        // When in a prompt mode, route input differently
        if self.input_mode != InputMode::Normal {
            self.handle_prompt_command(cmd);
            return;
        }

        match cmd {
            Command::Quit | Command::ForceQuit => {
                self.running = false;
            }
            Command::InsertChar(c) => self.cmd_insert_char(c),
            Command::InsertNewline => {
                self.handle_command(Command::InsertChar('\n'));
            }
            Command::DeleteBackward => self.cmd_delete_backward(),
            Command::DeleteForward => self.cmd_delete_forward(),
            Command::MoveLeft => self.cmd_move_left(),
            Command::MoveRight => self.cmd_move_right(),
            Command::MoveUp => self.cmd_move_up(),
            Command::MoveDown => self.cmd_move_down(),
            Command::MoveLineStart => self.cmd_move_line_start(),
            Command::MoveLineEnd => self.cmd_move_line_end(),
            Command::MoveBufferStart => self.cmd_move_buffer_start(),
            Command::MoveBufferEnd => self.cmd_move_buffer_end(),
            Command::PageUp => self.cmd_page_up(),
            Command::PageDown => self.cmd_page_down(),
            Command::Undo => {
                let _ = self.buffer.undo();
            }
            Command::Redo => {
                let _ = self.buffer.redo();
            }
            Command::Save => self.cmd_save(),
            Command::Open => {
                self.input_mode = InputMode::PromptOpen;
                self.prompt_input.clear();
            }
            Command::Find => {
                self.input_mode = InputMode::PromptFind;
                self.prompt_input.clear();
            }
            Command::FindNext => self.find_next(),
            Command::FindPrev => self.find_prev(),
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
                self.running = false;
            }
            Command::OpenFileFinder => self.cmd_open_file_finder(),
            Command::OpenCommandPalette => {
                self.messages.info("Command palette not yet implemented");
            }
            Command::MoveWordLeft => self.move_word_left(),
            Command::MoveWordRight => self.move_word_right(),
            Command::DeleteLine => self.delete_current_line(),
            Command::SelectAll => {
                self.buffer
                    .cursors_mut()
                    .primary_mut()
                    .set_position(Position::new(0, 0));
                self.messages.info("SelectAll: cursor moved to start");
            }
            // --- LSP commands ---
            Command::LspHover => self.lsp_hover(),
            Command::LspGotoDefinition => self.lsp_goto_definition(),
            Command::LspFindReferences => self.lsp_find_references(),
            Command::LspCompletion => self.lsp_completion(),
            Command::LspFormat => self.lsp_format(),
            Command::LspRename => {
                if self.lsp_server_started {
                    self.input_mode = InputMode::PromptLspRename;
                    self.prompt_input.clear();
                } else {
                    self.messages.warn("No LSP server running");
                }
            }
            Command::LspCodeAction => self.lsp_code_action(),
            Command::LspDiagnosticNext => self.lsp_diagnostic_next(),
            Command::LspDiagnosticPrev => self.lsp_diagnostic_prev(),
            Command::LspRestart => self.start_lsp_for_current_file(),
            // --- Jump navigation ---
            Command::JumpBack => self.cmd_jump_back(),
            Command::JumpForward => self.cmd_jump_forward(),
            _ => {
                // Commands not yet implemented in prototype
            }
        }
    }

    /// Handle input while a prompt is active.
    pub(crate) fn handle_prompt_command(&mut self, cmd: Command) {
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
                    InputMode::PromptLspRename => self.confirm_lsp_rename(&input),
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
}

// =========================================================================
// Individual command implementations
// =========================================================================

impl App {
    fn cmd_insert_char(&mut self, c: char) {
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
            self.lsp_did_change();
        }
    }

    fn cmd_delete_backward(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        if pos.col > 0 {
            let start = Position::new(pos.line, pos.col - 1);
            let range = smash_core::position::Range::new(start, pos);
            let edit = EditCommand::Delete { range };
            if self.buffer.apply_edit(edit).is_ok() {
                self.buffer.cursors_mut().primary_mut().set_position(start);
                self.lsp_did_change();
            }
        } else if pos.line > 0 {
            let prev_line = pos.line - 1;
            let prev_len = self
                .buffer
                .line(prev_line)
                .map(line_content_len)
                .unwrap_or(0);
            let start = Position::new(prev_line, prev_len);
            let range = smash_core::position::Range::new(start, pos);
            let edit = EditCommand::Delete { range };
            if self.buffer.apply_edit(edit).is_ok() {
                self.buffer.cursors_mut().primary_mut().set_position(start);
                self.lsp_did_change();
            }
        }
    }

    fn cmd_delete_forward(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        let line_len = self
            .buffer
            .line(pos.line)
            .map(line_content_len)
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
        if self.buffer.apply_edit(edit).is_ok() {
            self.lsp_did_change();
        }
    }

    fn cmd_move_left(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        if pos.col > 0 {
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(pos.line, pos.col - 1));
        }
    }

    fn cmd_move_right(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        let line_len = self
            .buffer
            .line(pos.line)
            .map(line_content_len)
            .unwrap_or(0);
        if pos.col < line_len {
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(pos.line, pos.col + 1));
        }
    }

    fn cmd_move_up(&mut self) {
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

    fn cmd_move_down(&mut self) {
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

    fn cmd_move_line_start(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(pos.line, 0));
    }

    fn cmd_move_line_end(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        let line_len = self
            .buffer
            .line(pos.line)
            .map(line_content_len)
            .unwrap_or(0);
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(pos.line, line_len));
    }

    fn cmd_move_buffer_start(&mut self) {
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(0, 0));
    }

    fn cmd_move_buffer_end(&mut self) {
        let last = self.buffer.line_count().saturating_sub(1);
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(last, 0));
    }

    fn cmd_page_up(&mut self) {
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

    fn cmd_page_down(&mut self) {
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

    fn cmd_save(&mut self) {
        if self.buffer.path().is_some() {
            match self.buffer.save() {
                Ok(()) => {
                    self.messages.info("File saved");
                    info!("file saved");
                    self.lsp_did_save();
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

    fn cmd_open_file_finder(&mut self) {
        self.input_mode = InputMode::FileFinder;
        self.prompt_input.clear();
        self.finder_results.clear();
        if self.file_finder.is_none() {
            if let Ok(cwd) = std::env::current_dir() {
                let mut finder = smash_core::fuzzy_finder::FileFinder::new(cwd);
                finder.index();
                self.file_finder = Some(finder);
            }
        }
    }
}

// =========================================================================
// Search & navigation helpers
// =========================================================================

impl App {
    /// Run incremental search on the current prompt input.
    pub(crate) fn incremental_search(&mut self) {
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
    pub(crate) fn confirm_open(&mut self, filename: &str) {
        let filename = filename.trim();
        if filename.is_empty() {
            self.messages.warn("Open cancelled — no filename entered");
            return;
        }
        let path = std::path::PathBuf::from(filename);
        let id = BufferId::next();
        match smash_core::buffer::Buffer::open_or_create(id, &path) {
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
    pub(crate) fn confirm_find(&mut self, query: &str) {
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
            self.find_next();
        } else {
            self.messages
                .warn(format!("No matches found for '{}'", query_str));
        }
    }

    /// Jump to the next search match.
    pub(crate) fn find_next(&mut self) {
        if let Some(m) = self.buffer.search_mut().next_match() {
            let pos = m.range.start;
            self.buffer.cursors_mut().primary_mut().set_position(pos);
        } else {
            self.messages.info("No search results");
        }
    }

    /// Jump to the previous search match.
    pub(crate) fn find_prev(&mut self) {
        if let Some(m) = self.buffer.search_mut().prev_match() {
            let pos = m.range.start;
            self.buffer.cursors_mut().primary_mut().set_position(pos);
        } else {
            self.messages.info("No search results");
        }
    }

    /// Confirm go-to-line: jump cursor to a specific line number.
    pub(crate) fn confirm_goto_line(&mut self, input: &str) {
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
    pub(crate) fn confirm_save_as(&mut self, input: &str) {
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
    pub(crate) fn confirm_find_replace(&mut self, pattern: &str, replacement: &str) {
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
    pub(crate) fn move_word_left(&mut self) {
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
                .map(line_content_len)
                .unwrap_or(0);
            self.buffer
                .cursors_mut()
                .primary_mut()
                .set_position(Position::new(prev_line, prev_len));
        }
    }

    /// Move cursor one word to the right.
    pub(crate) fn move_word_right(&mut self) {
        let pos = self.buffer.cursors().primary().position();
        if let Some(line_slice) = self.buffer.line(pos.line) {
            let len = line_content_len(line_slice);
            let line_str: String = line_slice.chars().collect();
            let chars: Vec<char> = line_str.chars().collect();
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
    pub(crate) fn delete_current_line(&mut self) {
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
                .map(line_content_len)
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
    pub(crate) fn update_finder_results(&mut self) {
        if let Some(ref finder) = self.file_finder {
            self.finder_results = finder.search(&self.prompt_input, 20);
        }
    }

    /// Confirm file finder selection: open the first result.
    pub(crate) fn confirm_file_finder(&mut self) {
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
}

// =========================================================================
// Jump navigation
// =========================================================================

impl App {
    /// Build a `JumpLocation` representing the current cursor position and file.
    pub(crate) fn current_jump_location(&self) -> JumpLocation {
        let path = self.buffer.path().map(|p| p.to_path_buf());
        let pos = self.buffer.cursors().primary().position();
        JumpLocation::new(path, pos)
    }

    /// Push the current location onto the jump stack (before a jump).
    pub(crate) fn push_jump(&mut self) {
        let loc = self.current_jump_location();
        self.jump_stack.push(loc);
    }

    /// Navigate to a `JumpLocation`, opening the file if necessary.
    fn navigate_to_location(&mut self, loc: &JumpLocation) {
        // If the target is in a different file, open it
        if loc.path != self.buffer.path().map(|p| p.to_path_buf()) {
            if let Some(ref path) = loc.path {
                self.confirm_open(&path.to_string_lossy());
            }
        }
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(loc.position);
    }

    /// Jump back to the previous location in the jump stack.
    fn cmd_jump_back(&mut self) {
        let current = self.current_jump_location();
        if let Some(loc) = self.jump_stack.pop_back(current) {
            let target = loc.clone();
            self.navigate_to_location(&target);
            self.messages.info("Jump back");
        } else {
            self.messages.info("No previous location");
        }
    }

    /// Jump forward to the next location in the forward stack.
    fn cmd_jump_forward(&mut self) {
        let current = self.current_jump_location();
        if let Some(loc) = self.jump_stack.pop_forward(current) {
            let target = loc.clone();
            self.navigate_to_location(&target);
            self.messages.info("Jump forward");
        } else {
            self.messages.info("No next location");
        }
    }
}
