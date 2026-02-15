use anyhow::Result;

use smash_lsp::DiagnosticSeverity;
use smash_tui::{default_dark_theme, Rect, TerminalBackend};

use super::{App, InputMode};

// =========================================================================
// Rendering
// =========================================================================

impl App {
    /// Return the highest-priority diagnostic severity for a buffer line.
    pub(crate) fn highest_diagnostic_severity(
        &self,
        buf_line: usize,
    ) -> Option<smash_tui::GutterDiagnostic> {
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

    pub(crate) fn render(&mut self, backend: &mut dyn TerminalBackend) -> Result<()> {
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

        // Render status bar based on current input mode
        self.render_status_bar(status_area, pos, &theme);

        self.renderer.flush_to_backend(backend)?;

        let gutter_w = 7u16;
        let screen_col = gutter_w + (pos.col.saturating_sub(self.viewport.left_col())) as u16;
        let screen_row = (pos.line.saturating_sub(self.viewport.top_line())) as u16;
        backend.move_cursor(screen_col, screen_row)?;
        backend.show_cursor()?;

        Ok(())
    }

    /// Render the status bar content based on the current input mode.
    fn render_status_bar(
        &mut self,
        status_area: Rect,
        pos: smash_core::position::Position,
        theme: &smash_tui::Theme,
    ) {
        match &self.input_mode {
            InputMode::PromptOpen => {
                let prompt_text = format!("Open file: {}", self.prompt_input);
                self.renderer.render_status_bar(
                    status_area,
                    &prompt_text,
                    pos.line,
                    pos.col,
                    false,
                    theme,
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
                    theme,
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
                    theme,
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
                    theme,
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
                    theme,
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
                    theme,
                );
            }
            InputMode::Normal => {
                let status_text = self.build_normal_status_text();
                self.renderer.render_status_bar(
                    status_area,
                    &status_text,
                    pos.line,
                    pos.col,
                    self.buffer.is_dirty(),
                    theme,
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
                    theme,
                );
            }
        }
    }

    /// Build the status text for Normal mode (includes LSP info, diagnostics).
    fn build_normal_status_text(&self) -> String {
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

        if let Some(msg) = self.messages.last() {
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
        }
    }
}
