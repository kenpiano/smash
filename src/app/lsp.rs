use smash_core::edit::EditCommand;
use smash_core::position::Position;
use smash_lsp::{DiagnosticSeverity, LspPosition, LspRange, LspServerConfig};
use tracing::info;

use super::{App, InputMode};
use crate::lsp_types::{LspCommand, LspEvent};

// =========================================================================
// LSP integration helpers
// =========================================================================

impl App {
    /// Convert a filesystem path to a `file://` URI.
    ///
    /// Relative paths are resolved against the current working directory
    /// so the URI always contains an absolute path — a requirement of the
    /// LSP specification.
    pub(crate) fn path_to_uri(path: &std::path::Path) -> String {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        format!("file://{}", abs.to_string_lossy())
    }

    /// Get the file URI for the current buffer.
    pub(crate) fn current_uri(&self) -> Option<String> {
        self.buffer.path().map(Self::path_to_uri)
    }

    /// Start an LSP server for the current file's language, if configured.
    pub(crate) fn start_lsp_for_current_file(&mut self) {
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
    pub(crate) fn lsp_did_open(&self) {
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
    pub(crate) fn lsp_did_change(&mut self) {
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
    pub(crate) fn lsp_did_save(&self) {
        if !self.lsp_server_started {
            return;
        }
        if let Some(uri) = self.current_uri() {
            let _ = self.lsp_cmd_tx.try_send(LspCommand::DidSave { uri });
        }
    }

    /// Request hover information at the cursor position.
    pub(crate) fn lsp_hover(&mut self) {
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
    pub(crate) fn lsp_goto_definition(&mut self) {
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
    pub(crate) fn lsp_find_references(&mut self) {
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
    pub(crate) fn lsp_completion(&mut self) {
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
    pub(crate) fn lsp_format(&mut self) {
        if !self.lsp_server_started {
            self.messages.warn("No LSP server running");
            return;
        }
        if let Some(uri) = self.current_uri() {
            let _ = self.lsp_cmd_tx.try_send(LspCommand::Format { uri });
        }
    }

    /// Request code actions at the cursor position.
    pub(crate) fn lsp_code_action(&mut self) {
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
    pub(crate) fn lsp_diagnostic_next(&mut self) {
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
    pub(crate) fn lsp_diagnostic_prev(&mut self) {
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
    pub(crate) fn confirm_lsp_rename(&mut self, new_name: &str) {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.messages.warn("Rename cancelled — no name entered");
            self.input_mode = InputMode::Normal;
            return;
        }
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
}

// =========================================================================
// LSP event handling
// =========================================================================

impl App {
    /// Handle LSP events received from the async task.
    pub(crate) fn handle_lsp_event(&mut self, event: LspEvent) {
        match event {
            LspEvent::ServerStarted(lang) => {
                self.lsp_server_started = true;
                self.messages
                    .info(format!("LSP server started for {}", lang));
                info!(language = %lang, "LSP server started");
                self.lsp_did_open();
            }
            LspEvent::HoverResult(text) => self.handle_hover_result(text),
            LspEvent::GotoDefinitionResult(locations) => {
                self.handle_goto_definition_result(locations);
            }
            LspEvent::ReferencesResult(locations) => self.handle_references_result(locations),
            LspEvent::CompletionResult(items) => self.handle_completion_result(items),
            LspEvent::FormatResult(edits) => self.handle_format_result(edits),
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
                self.handle_diagnostics_updated(uri, diagnostics);
            }
            LspEvent::Error(msg) => {
                self.messages.error(format!("LSP: {}", msg));
                tracing::error!(msg = %msg, "LSP error");
            }
            LspEvent::Info(msg) => {
                self.messages.info(format!("LSP: {}", msg));
            }
        }
    }

    fn handle_hover_result(&mut self, text: Option<String>) {
        if let Some(text) = text {
            let display = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text.clone()
            };
            let display = display.replace('\n', " | ");
            self.hover_text = Some(text);
            self.messages.info(format!("Hover: {}", display));
        } else {
            self.hover_text = None;
            self.messages.info("No hover information");
        }
    }

    fn handle_goto_definition_result(&mut self, locations: Vec<smash_lsp::Location>) {
        if locations.is_empty() {
            self.messages.info("No definition found");
            return;
        }

        // Save current position before jumping
        self.push_jump();

        let loc = &locations[0];
        let line = loc.range.start.line as usize;
        let col = loc.range.start.character as usize;

        // Check if it's a different file
        let current_uri = self.current_uri().unwrap_or_default();
        if loc.uri != current_uri {
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

    fn handle_references_result(&mut self, locations: Vec<smash_lsp::Location>) {
        if locations.is_empty() {
            self.messages.info("No references found");
            return;
        }

        // Save current position before jumping
        self.push_jump();

        let count = locations.len();
        let loc = &locations[0];
        let line = loc.range.start.line as usize;
        let col = loc.range.start.character as usize;

        // Check if it's a different file
        let current_uri = self.current_uri().unwrap_or_default();
        if loc.uri != current_uri {
            let path = loc.uri.strip_prefix("file://").unwrap_or(&loc.uri);
            self.confirm_open(path);
        }

        self.buffer
            .cursors_mut()
            .primary_mut()
            .set_position(Position::new(line, col));
        self.messages.info(format!("Found {} reference(s)", count));
    }

    fn handle_completion_result(&mut self, items: Vec<smash_lsp::CompletionItem>) {
        if items.is_empty() {
            self.messages.info("No completions");
            self.completion_items.clear();
            return;
        }
        let count = items.len();
        let preview: Vec<&str> = items.iter().take(5).map(|i| i.label.as_str()).collect();
        self.messages.info(format!(
            "Completions ({}): {}{}",
            count,
            preview.join(", "),
            if count > 5 { ", ..." } else { "" }
        ));
        self.completion_items = items;
        self.completion_index = 0;
    }

    fn handle_format_result(&mut self, edits: Vec<smash_lsp::TextEdit>) {
        if edits.is_empty() {
            self.messages.info("No formatting changes");
            return;
        }
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

    fn handle_diagnostics_updated(&mut self, uri: String, diagnostics: Vec<smash_lsp::Diagnostic>) {
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
}
