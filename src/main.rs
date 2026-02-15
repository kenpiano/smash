use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use tokio::sync::Mutex as TokioMutex;
use tracing::{error, info};

use smash_config::load_config;
use smash_core::buffer::{Buffer, BufferId};
use smash_core::edit::EditCommand;
use smash_core::message::MessageBuffer;
use smash_core::position::Position;
use smash_core::search::SearchQuery;
use smash_input::{
    create_default_keymap, create_emacs_keymap, Command, KeyResolver, Keymap, ResolveResult,
};
use smash_lsp::{
    CompletionItem, Diagnostic, DiagnosticSeverity, LspPosition, LspRange, LspRegistry,
    LspServerConfig,
};
use smash_platform::paths::DefaultPaths;
use smash_platform::paths::PlatformPaths;
use smash_platform::Platform;
use smash_syntax::{LanguageId, RegexHighlighter};
use smash_tui::{default_dark_theme, PaneTree, Rect, Renderer, TerminalBackend, Viewport};

/// Return the "content length" of a rope line slice.
///
/// Ropey includes a trailing `\n` in `len_chars()` for every line except
/// (potentially) the last one.  This helper subtracts 1 only when the line
/// actually ends with a newline so that the cursor can reach the true end
/// of text on the final line.
fn line_content_len(line: smash_core::buffer::RopeSlice<'_>) -> usize {
    let len = line.len_chars();
    if len > 0 && line.char(len - 1) == '\n' {
        len - 1
    } else {
        len
    }
}

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
    /// Prompt for LSP rename (new symbol name).
    PromptLspRename,
}

/// Events sent from the async LSP task back to the main thread.
#[allow(dead_code)]
enum LspEvent {
    /// LSP server started for a language.
    ServerStarted(String),
    /// Hover result (text to display).
    HoverResult(Option<String>),
    /// Go-to-definition result (locations).
    GotoDefinitionResult(Vec<smash_lsp::Location>),
    /// Find-references result.
    ReferencesResult(Vec<smash_lsp::Location>),
    /// Completion result.
    CompletionResult(Vec<CompletionItem>),
    /// Format result (text edits).
    FormatResult(Vec<smash_lsp::TextEdit>),
    /// Code actions available.
    CodeActionResult(Vec<smash_lsp::CodeAction>),
    /// Diagnostics updated for a URI.
    DiagnosticsUpdated {
        uri: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// Error message from LSP.
    Error(String),
    /// Info message from LSP.
    Info(String),
}

/// Commands sent from the main thread to the async LSP task.
#[allow(dead_code)]
enum LspCommand {
    StartServer(LspServerConfig),
    DidOpen {
        uri: String,
        text: String,
        language_id: String,
    },
    DidChange {
        uri: String,
        version: i32,
        text: String,
    },
    DidSave {
        uri: String,
    },
    DidClose {
        uri: String,
    },
    Hover {
        uri: String,
        position: LspPosition,
    },
    GotoDefinition {
        uri: String,
        position: LspPosition,
    },
    FindReferences {
        uri: String,
        position: LspPosition,
    },
    Completion {
        uri: String,
        position: LspPosition,
    },
    Format {
        uri: String,
    },
    CodeAction {
        uri: String,
        range: LspRange,
    },
    Shutdown,
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
    // --- LSP integration ---
    /// Channel to send commands to the LSP async task.
    lsp_cmd_tx: tokio::sync::mpsc::Sender<LspCommand>,
    /// Channel to receive events from the LSP async task.
    lsp_evt_rx: std::sync::mpsc::Receiver<LspEvent>,
    /// Current document version (incremented on each edit for didChange).
    document_version: i32,
    /// Current language ID for the active buffer.
    language_id: Option<String>,
    /// Whether LSP is enabled in config.
    lsp_enabled: bool,
    /// LSP server configs from config file.
    lsp_server_configs: std::collections::HashMap<String, smash_config::LspServerEntry>,
    /// Whether an LSP server has been started for the current language.
    lsp_server_started: bool,
    /// Diagnostics for the current file.
    current_diagnostics: Vec<Diagnostic>,
    /// Current diagnostic index for next/prev navigation.
    diagnostic_index: usize,
    /// Last hover text to display.
    hover_text: Option<String>,
    /// Completion items from LSP.
    completion_items: Vec<CompletionItem>,
    /// Selected completion index.
    completion_index: usize,
    /// Whether to normalize macOS Option key to Alt.
    option_as_alt: bool,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    fn new(
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
                    self.lsp_did_change();
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
            Command::DeleteForward => {
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
                    .map(line_content_len)
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
                    .map(line_content_len)
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
            // --- LSP commands ---
            Command::LspHover => {
                self.lsp_hover();
            }
            Command::LspGotoDefinition => {
                self.lsp_goto_definition();
            }
            Command::LspFindReferences => {
                self.lsp_find_references();
            }
            Command::LspCompletion => {
                self.lsp_completion();
            }
            Command::LspFormat => {
                self.lsp_format();
            }
            Command::LspRename => {
                if self.lsp_server_started {
                    self.input_mode = InputMode::PromptLspRename;
                    self.prompt_input.clear();
                } else {
                    self.messages.warn("No LSP server running");
                }
            }
            Command::LspCodeAction => {
                self.lsp_code_action();
            }
            Command::LspDiagnosticNext => {
                self.lsp_diagnostic_next();
            }
            Command::LspDiagnosticPrev => {
                self.lsp_diagnostic_prev();
            }
            Command::LspRestart => {
                self.start_lsp_for_current_file();
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
                .map(line_content_len)
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

    // =====================================================================
    // LSP integration helpers
    // =====================================================================

    /// Convert a filesystem path to a `file://` URI.
    ///
    /// Relative paths are resolved against the current working directory
    /// so the URI always contains an absolute path — a requirement of the
    /// LSP specification.
    fn path_to_uri(path: &std::path::Path) -> String {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        format!("file://{}", abs.to_string_lossy())
    }

    /// Get the file URI for the current buffer.
    fn current_uri(&self) -> Option<String> {
        self.buffer.path().map(Self::path_to_uri)
    }

    /// Start an LSP server for the current file's language, if configured.
    fn start_lsp_for_current_file(&mut self) {
        if !self.lsp_enabled {
            return;
        }
        let lang_id = match &self.language_id {
            Some(l) => l.clone(),
            None => return,
        };

        if let Some(entry) = self.lsp_server_configs.get(&lang_id) {
            let root_uri = std::env::current_dir().ok().map(|p| Self::path_to_uri(&p));
            let config = LspServerConfig {
                command: entry.command.clone(),
                args: entry.args.clone(),
                language_id: lang_id.clone(),
                root_uri,
            };
            let _ = self.lsp_cmd_tx.try_send(LspCommand::StartServer(config));
            info!(language = %lang_id, "requesting LSP server start");
        }
    }

    /// Send didOpen notification for the current buffer.
    fn lsp_did_open(&self) {
        if !self.lsp_server_started {
            return;
        }
        let uri = match self.current_uri() {
            Some(u) => u,
            None => return,
        };
        let lang_id = match &self.language_id {
            Some(l) => l.clone(),
            None => return,
        };
        let text = self.buffer.text().to_string();
        let _ = self.lsp_cmd_tx.try_send(LspCommand::DidOpen {
            uri,
            text,
            language_id: lang_id,
        });
    }

    /// Send didChange notification after an edit.
    fn lsp_did_change(&mut self) {
        if !self.lsp_server_started {
            return;
        }
        let uri = match self.current_uri() {
            Some(u) => u,
            None => return,
        };
        self.document_version += 1;
        let text = self.buffer.text().to_string();
        let _ = self.lsp_cmd_tx.try_send(LspCommand::DidChange {
            uri,
            version: self.document_version,
            text,
        });
    }

    /// Send didSave notification.
    fn lsp_did_save(&self) {
        if !self.lsp_server_started {
            return;
        }
        if let Some(uri) = self.current_uri() {
            let _ = self.lsp_cmd_tx.try_send(LspCommand::DidSave { uri });
        }
    }

    /// Request hover information at the cursor position.
    fn lsp_hover(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let _ = self.lsp_cmd_tx.try_send(LspCommand::Hover {
                uri,
                position: LspPosition::from(pos),
            });
        }
    }

    /// Request go-to-definition at the cursor position.
    fn lsp_goto_definition(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let _ = self.lsp_cmd_tx.try_send(LspCommand::GotoDefinition {
                uri,
                position: LspPosition::from(pos),
            });
        }
    }

    /// Request find-references at the cursor position.
    fn lsp_find_references(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let _ = self.lsp_cmd_tx.try_send(LspCommand::FindReferences {
                uri,
                position: LspPosition::from(pos),
            });
        }
    }

    /// Request completions at the cursor position.
    fn lsp_completion(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let _ = self.lsp_cmd_tx.try_send(LspCommand::Completion {
                uri,
                position: LspPosition::from(pos),
            });
        }
    }

    /// Request document formatting.
    fn lsp_format(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let _ = self.lsp_cmd_tx.try_send(LspCommand::Format { uri });
        }
    }

    /// Request code actions at the cursor position.
    fn lsp_code_action(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let range = LspRange::new(LspPosition::from(pos), LspPosition::from(pos));
            let _ = self
                .lsp_cmd_tx
                .try_send(LspCommand::CodeAction { uri, range });
        }
    }

    /// Navigate to the next diagnostic.
    fn lsp_diagnostic_next(&mut self) {
        if self.current_diagnostics.is_empty() {
            self.messages.info("No diagnostics");
            return;
        }
        if self.diagnostic_index >= self.current_diagnostics.len() {
            self.diagnostic_index = 0;
        }
        let diag = &self.current_diagnostics[self.diagnostic_index];
        let line = diag.range.start.line as usize;
        let col = diag.range.start.character as usize;
        let severity = match diag.severity {
            Some(DiagnosticSeverity::Error) => "error",
            Some(DiagnosticSeverity::Warning) => "warning",
            Some(DiagnosticSeverity::Information) => "info",
            Some(DiagnosticSeverity::Hint) => "hint",
            None => "diagnostic",
        };
        let msg = format!(
            "[{}/{}] {}: {}",
            self.diagnostic_index + 1,
            self.current_diagnostics.len(),
            severity,
            diag.message
        );
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(line, col));
        self.messages.info(msg);
        self.diagnostic_index += 1;
    }

    /// Navigate to the previous diagnostic.
    fn lsp_diagnostic_prev(&mut self) {
        if self.current_diagnostics.is_empty() {
            self.messages.info("No diagnostics");
            return;
        }
        if self.diagnostic_index == 0 {
            self.diagnostic_index = self.current_diagnostics.len();
        }
        self.diagnostic_index -= 1;
        let diag = &self.current_diagnostics[self.diagnostic_index];
        let line = diag.range.start.line as usize;
        let col = diag.range.start.character as usize;
        let severity = match diag.severity {
            Some(DiagnosticSeverity::Error) => "error",
            Some(DiagnosticSeverity::Warning) => "warning",
            Some(DiagnosticSeverity::Information) => "info",
            Some(DiagnosticSeverity::Hint) => "hint",
            None => "diagnostic",
        };
        let msg = format!(
            "[{}/{}] {}: {}",
            self.diagnostic_index + 1,
            self.current_diagnostics.len(),
            severity,
            diag.message
        );
        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(line, col));
        self.messages.info(msg);
    }

    /// Confirm LSP rename from prompt.
    fn confirm_lsp_rename(&mut self, new_name: &str) {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.messages.warn("Rename cancelled — no name entered");
            self.input_mode = InputMode::Normal;
            return;
        }
        // Rename is handled by sending to the LSP — but the rename response
        // comes back as a WorkspaceEdit which we'd need to apply. For now,
        // show a message that the rename was requested.
        if let Some(_uri) = self.current_uri() {
            let pos = self.buffer.cursors().primary().position();
            let lsp_pos = LspPosition::from(pos);
            self.messages.info(format!(
                "Rename requested: '{}' at {}:{}",
                new_name,
                lsp_pos.line + 1,
                lsp_pos.character + 1,
            ));
            info!(new_name = %new_name, "LSP rename requested");
            // TODO: Send rename request to LSP when we add LspCommand::Rename
        } else {
            self.messages.warn("No file path for rename");
        }
        self.input_mode = InputMode::Normal;
    }

    /// Handle LSP events received from the async task.
    fn handle_lsp_event(&mut self, event: LspEvent) {
        match event {
            LspEvent::ServerStarted(lang) => {
                self.lsp_server_started = true;
                self.messages
                    .info(format!("LSP server started for {}", lang));
                info!(language = %lang, "LSP server started");
                // Send didOpen for the current file
                self.lsp_did_open();
            }
            LspEvent::HoverResult(text) => {
                if let Some(text) = text {
                    // Truncate long hover text for status bar
                    let display = if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text.clone()
                    };
                    // Replace newlines with spaces for single-line display
                    let display = display.replace('\n', " | ");
                    self.hover_text = Some(text);
                    self.messages.info(format!("Hover: {}", display));
                } else {
                    self.hover_text = None;
                    self.messages.info("No hover information");
                }
            }
            LspEvent::GotoDefinitionResult(locations) => {
                if locations.is_empty() {
                    self.messages.info("No definition found");
                } else {
                    let loc = &locations[0];
                    let line = loc.range.start.line as usize;
                    let col = loc.range.start.character as usize;

                    // Check if it's a different file
                    let current_uri = self.current_uri().unwrap_or_default();
                    if loc.uri != current_uri {
                        // Try to open the file
                        let path = loc.uri.strip_prefix("file://").unwrap_or(&loc.uri);
                        self.confirm_open(path);
                    }

                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(line, col));

                    if locations.len() > 1 {
                        self.messages.info(format!(
                            "Definition: {}:{} ({} locations)",
                            line + 1,
                            col + 1,
                            locations.len()
                        ));
                    } else {
                        self.messages
                            .info(format!("Definition: {}:{}", line + 1, col + 1));
                    }
                }
            }
            LspEvent::ReferencesResult(locations) => {
                if locations.is_empty() {
                    self.messages.info("No references found");
                } else {
                    let count = locations.len();
                    // Jump to first reference
                    let loc = &locations[0];
                    let line = loc.range.start.line as usize;
                    let col = loc.range.start.character as usize;
                    self.buffer
                        .cursors_mut()
                        .primary_mut()
                        .set_position(Position::new(line, col));
                    self.messages.info(format!("Found {} reference(s)", count));
                }
            }
            LspEvent::CompletionResult(items) => {
                if items.is_empty() {
                    self.messages.info("No completions");
                    self.completion_items.clear();
                } else {
                    let count = items.len();
                    let preview: Vec<&str> =
                        items.iter().take(5).map(|i| i.label.as_str()).collect();
                    self.messages.info(format!(
                        "Completions ({}): {}{}",
                        count,
                        preview.join(", "),
                        if count > 5 { ", ..." } else { "" }
                    ));
                    self.completion_items = items;
                    self.completion_index = 0;
                }
            }
            LspEvent::FormatResult(edits) => {
                if edits.is_empty() {
                    self.messages.info("No formatting changes");
                } else {
                    // Apply text edits in reverse order to preserve positions
                    let mut edits = edits;
                    edits.sort_by(|a, b| {
                        b.range
                            .start
                            .line
                            .cmp(&a.range.start.line)
                            .then(b.range.start.character.cmp(&a.range.start.character))
                    });
                    let mut applied = 0;
                    for edit in &edits {
                        let start = Position::new(
                            edit.range.start.line as usize,
                            edit.range.start.character as usize,
                        );
                        let end = Position::new(
                            edit.range.end.line as usize,
                            edit.range.end.character as usize,
                        );
                        let range = smash_core::position::Range::new(start, end);
                        let delete = EditCommand::Delete { range };
                        if self.buffer.apply_edit(delete).is_ok() {
                            let insert = EditCommand::Insert {
                                pos: start,
                                text: edit.new_text.clone(),
                            };
                            let _ = self.buffer.apply_edit(insert);
                            applied += 1;
                        }
                    }
                    self.messages
                        .info(format!("Applied {} formatting edit(s)", applied));
                    self.lsp_did_change();
                }
            }
            LspEvent::CodeActionResult(actions) => {
                if actions.is_empty() {
                    self.messages.info("No code actions available");
                } else {
                    let titles: Vec<&str> =
                        actions.iter().take(5).map(|a| a.title.as_str()).collect();
                    self.messages
                        .info(format!("Code actions: {}", titles.join(", ")));
                    // TODO: Allow selecting and applying code actions
                }
            }
            LspEvent::DiagnosticsUpdated { uri, diagnostics } => {
                let current_uri = self.current_uri().unwrap_or_default();
                if uri == current_uri {
                    let count = diagnostics.len();
                    let errors = diagnostics
                        .iter()
                        .filter(|d| d.severity == Some(DiagnosticSeverity::Error))
                        .count();
                    let warnings = diagnostics
                        .iter()
                        .filter(|d| d.severity == Some(DiagnosticSeverity::Warning))
                        .count();
                    self.current_diagnostics = diagnostics;
                    self.diagnostic_index = 0;
                    if count > 0 {
                        self.messages.info(format!(
                            "Diagnostics: {} error(s), {} warning(s), {} total",
                            errors, warnings, count
                        ));
                    }
                }
            }
            LspEvent::Error(msg) => {
                self.messages.error(format!("LSP: {}", msg));
                error!(msg = %msg, "LSP error");
            }
            LspEvent::Info(msg) => {
                self.messages.info(format!("LSP: {}", msg));
            }
        }
    }

    /// Return the highest-priority diagnostic severity for a buffer line.
    fn highest_diagnostic_severity(&self, buf_line: usize) -> Option<smash_tui::GutterDiagnostic> {
        use smash_lsp::types::DiagnosticSeverity;

        let mut worst: Option<DiagnosticSeverity> = None;
        for diag in &self.current_diagnostics {
            if diag.range.start.line as usize <= buf_line
                && diag.range.end.line as usize >= buf_line
            {
                let sev = diag.severity.unwrap_or(DiagnosticSeverity::Error);
                worst = Some(match worst {
                    None => sev,
                    Some(cur) if (sev as i32) < (cur as i32) => sev,
                    Some(cur) => cur,
                });
            }
        }
        worst.map(|s| match s {
            DiagnosticSeverity::Error => smash_tui::GutterDiagnostic::Error,
            DiagnosticSeverity::Warning => smash_tui::GutterDiagnostic::Warning,
            DiagnosticSeverity::Information => smash_tui::GutterDiagnostic::Information,
            DiagnosticSeverity::Hint => smash_tui::GutterDiagnostic::Hint,
        })
    }

    fn render(&mut self, backend: &mut dyn TerminalBackend) -> Result<()> {
        let (w, h) = backend.size()?;

        let pos = self.buffer.cursors().primary().position();
        self.viewport.scroll_to_cursor(pos.line, pos.col);

        let status_h = 1u16;
        let edit_area = Rect::new(0, 0, w, h.saturating_sub(status_h));
        let status_area = Rect::new(0, h.saturating_sub(status_h), w, status_h);

        let theme = default_dark_theme();

        // Build per-screen-row diagnostic severity map for the gutter.
        let line_diagnostics: Vec<Option<smash_tui::GutterDiagnostic>> = (0..edit_area.height)
            .map(|row| {
                let buf_line = self.viewport.top_line() + row as usize;
                self.highest_diagnostic_severity(buf_line)
            })
            .collect();

        self.renderer.render_buffer(
            &self.buffer,
            &self.viewport,
            edit_area,
            &theme,
            self.highlighter
                .as_ref()
                .map(|h| h as &dyn smash_syntax::HighlightEngine),
            true,
            &line_diagnostics,
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
                // Build status text with LSP info
                let diag_info = if !self.current_diagnostics.is_empty() {
                    let errors = self
                        .current_diagnostics
                        .iter()
                        .filter(|d| d.severity == Some(DiagnosticSeverity::Error))
                        .count();
                    let warnings = self
                        .current_diagnostics
                        .iter()
                        .filter(|d| d.severity == Some(DiagnosticSeverity::Warning))
                        .count();
                    format!(" E:{} W:{}", errors, warnings)
                } else {
                    String::new()
                };

                let lsp_indicator = if self.lsp_server_started {
                    " [LSP]"
                } else {
                    ""
                };

                // Show last message if recent, otherwise normal status
                let status_text = if let Some(msg) = self.messages.last() {
                    format!(
                        "{}{}{} | {}",
                        self.filename.as_deref().unwrap_or("[scratch]"),
                        lsp_indicator,
                        diag_info,
                        msg.text()
                    )
                } else {
                    format!(
                        "{}{}{}",
                        self.filename.as_deref().unwrap_or("[scratch]"),
                        lsp_indicator,
                        diag_info,
                    )
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
            InputMode::PromptLspRename => {
                let prompt_text = format!("Rename to: {}", self.prompt_input);
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    &theme,
                );
            }
        }

        self.renderer.flush_to_backend(backend)?;

        let gutter_w = 7u16;
        let screen_col = gutter_w + (pos.col.saturating_sub(self.viewport.left_col())) as u16;
        let screen_row = (pos.line.saturating_sub(self.viewport.top_line())) as u16;
        backend.move_cursor(screen_col, screen_row)?;
        backend.show_cursor()?;

        Ok(())
    }
}

fn run_editor(file: Option<PathBuf>) -> Result<()> {
    let paths = DefaultPaths::new().context("failed to detect platform paths")?;

    // Load configuration first so we can honour log settings.
    let config_dir = paths.config_dir();
    let project_dir = std::env::current_dir().ok();
    let config = load_config(&config_dir, project_dir.as_deref()).unwrap_or_else(|_e| {
        // Cannot use tracing yet — subscriber isn't initialised.
        smash_config::Config::default()
    });

    // ── Logging initialisation (REQ-NFR-020, REQ-NFR-021) ──────────────────
    //
    // Resolve the log file path: explicit config value → platform default.
    let log_path = config.log.file.clone().unwrap_or_else(|| {
        let dir = paths.log_dir();
        dir.join("smash.log")
    });

    // Ensure parent directory exists and rotate if the file is too large.
    smash_core::logging::ensure_log_dir(&log_path).ok();
    smash_core::logging::rotate_log_files(
        &log_path,
        smash_core::logging::DEFAULT_MAX_LOG_SIZE,
        smash_core::logging::DEFAULT_MAX_LOG_FILES,
    )
    .ok();

    // Determine the tracing filter level from config.
    let filter_str = match &config.log.level {
        smash_config::config::LogLevel::Trace => "trace",
        smash_config::config::LogLevel::Debug => "debug",
        smash_config::config::LogLevel::Info => "info",
        smash_config::config::LogLevel::Warn => "warn",
        smash_config::config::LogLevel::Error => "error",
    };

    // Open log file (append, not truncate) so rotated history is preserved.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap_or_else(|_| std::fs::File::create("/dev/null").expect("cannot open /dev/null"));

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter_str)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false)
        .with_env_filter(env_filter)
        .init();

    info!("smash starting – log level: {}", filter_str);

    let _platform = Platform::default_platform().context("failed to initialize platform")?;

    let (width, height) = crossterm::terminal::size().context("failed to get terminal size")?;

    // Set up LSP channels
    let (lsp_cmd_tx, lsp_cmd_rx) = tokio::sync::mpsc::channel::<LspCommand>(64);
    let (lsp_evt_tx, lsp_evt_rx) = std::sync::mpsc::channel::<LspEvent>();

    // Start tokio runtime for async LSP operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    // Spawn the LSP manager task
    runtime.spawn(lsp_manager_task(lsp_cmd_rx, lsp_evt_tx));

    let mut app = App::new(
        width,
        height,
        file,
        &config.keymap.preset,
        lsp_cmd_tx.clone(),
        lsp_evt_rx,
        config.lsp.enabled,
        config.lsp.servers.clone(),
        config.editor.option_as_alt,
    )?;

    // Start LSP for initial file if configured
    app.start_lsp_for_current_file();

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
        // Drain any pending LSP events
        let mut had_lsp_event = false;
        while let Ok(evt) = app.lsp_evt_rx.try_recv() {
            app.handle_lsp_event(evt);
            had_lsp_event = true;
        }
        if had_lsp_event {
            if let Err(e) = app.render(&mut backend) {
                error!("render error: {}", e);
            }
        }

        if event::poll(Duration::from_millis(50))? {
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
                // Normalize macOS Option key to Alt when configured
                let input = if app.option_as_alt {
                    match input {
                        smash_input::InputEvent::Key(ke) => smash_input::InputEvent::Key(
                            smash_input::event::normalize_macos_option_key(ke),
                        ),
                        other => other,
                    }
                } else {
                    input
                };
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

    // Shutdown LSP servers
    let _ = lsp_cmd_tx.try_send(LspCommand::Shutdown);
    // Give the runtime a moment to clean up
    drop(lsp_cmd_tx);
    runtime.shutdown_timeout(Duration::from_secs(2));

    info!("smash exited cleanly");
    Ok(())
}

/// Async task that manages LSP servers and processes commands.
///
/// Runs on the tokio runtime, receives commands from the main thread,
/// and sends events back via the event channel.
async fn lsp_manager_task(
    mut cmd_rx: tokio::sync::mpsc::Receiver<LspCommand>,
    evt_tx: std::sync::mpsc::Sender<LspEvent>,
) {
    let registry = Arc::new(TokioMutex::new(LspRegistry::new()));

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            LspCommand::StartServer(config) => {
                let lang = config.language_id.clone();
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let mut reg = registry.lock().await;
                    match reg.start_server(config).await {
                        Ok(_id) => {
                            // Wire diagnostic callback so updates reach the UI.
                            if let Some(client) = reg.get(&lang) {
                                let diag_store = client.diagnostics();
                                let diag_tx = evt_tx.clone();
                                diag_store.lock().await.set_on_update(move |uri, diags| {
                                    let _ = diag_tx.send(LspEvent::DiagnosticsUpdated {
                                        uri: uri.to_string(),
                                        diagnostics: diags.to_vec(),
                                    });
                                });
                            }
                            let _ = evt_tx.send(LspEvent::ServerStarted(lang));
                        }
                        Err(e) => {
                            let _ = evt_tx.send(LspEvent::Error(format!(
                                "Failed to start LSP for {}: {}",
                                lang, e
                            )));
                        }
                    }
                });
            }
            LspCommand::DidOpen {
                uri,
                text,
                language_id,
            } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                let lang_id = language_id.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    if let Some(client) = reg.get(&lang_id) {
                        if let Err(e) = client.did_open(&uri, &text, &lang_id).await {
                            let _ = evt_tx.send(LspEvent::Error(format!("didOpen: {}", e)));
                        }
                    }
                });
            }
            LspCommand::DidChange { uri, version, text } => {
                let registry = registry.clone();
                // Find the language for this URI by checking all clients
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            let _ = client.did_change(&uri, version, &text).await;
                            break;
                        }
                    }
                });
            }
            LspCommand::DidSave { uri } => {
                let registry = registry.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            let _ = client.did_save(&uri).await;
                            break;
                        }
                    }
                });
            }
            LspCommand::DidClose { uri } => {
                let registry = registry.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            let _ = client.did_close(&uri).await;
                            break;
                        }
                    }
                });
            }
            LspCommand::Hover { uri, position } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.hover(&uri, position).await {
                                Ok(hover) => {
                                    let text = hover.map(|h| h.contents.value);
                                    let _ = evt_tx.send(LspEvent::HoverResult(text));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(LspEvent::Error(format!("hover: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::GotoDefinition { uri, position } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.goto_definition(&uri, position).await {
                                Ok(locations) => {
                                    let _ = evt_tx.send(LspEvent::GotoDefinitionResult(locations));
                                }
                                Err(e) => {
                                    let _ = evt_tx
                                        .send(LspEvent::Error(format!("gotoDefinition: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::FindReferences { uri, position } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.find_references(&uri, position).await {
                                Ok(locations) => {
                                    let _ = evt_tx.send(LspEvent::ReferencesResult(locations));
                                }
                                Err(e) => {
                                    let _ = evt_tx
                                        .send(LspEvent::Error(format!("findReferences: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::Completion { uri, position } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.completion(&uri, position).await {
                                Ok(items) => {
                                    let _ = evt_tx.send(LspEvent::CompletionResult(items));
                                }
                                Err(e) => {
                                    let _ =
                                        evt_tx.send(LspEvent::Error(format!("completion: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::Format { uri } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.format(&uri).await {
                                Ok(edits) => {
                                    let _ = evt_tx.send(LspEvent::FormatResult(edits));
                                }
                                Err(e) => {
                                    let _ = evt_tx.send(LspEvent::Error(format!("format: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::CodeAction { uri, range } => {
                let registry = registry.clone();
                let evt_tx = evt_tx.clone();
                tokio::spawn(async move {
                    let reg = registry.lock().await;
                    for lang in reg.active_languages() {
                        if let Some(client) = reg.get(lang) {
                            match client.code_action(&uri, range, vec![]).await {
                                Ok(actions) => {
                                    let _ = evt_tx.send(LspEvent::CodeActionResult(actions));
                                }
                                Err(e) => {
                                    let _ =
                                        evt_tx.send(LspEvent::Error(format!("codeAction: {}", e)));
                                }
                            }
                            break;
                        }
                    }
                });
            }
            LspCommand::Shutdown => {
                let mut reg = registry.lock().await;
                reg.shutdown_all().await;
                break;
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use smash_core::buffer::{Buffer, BufferId};

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
