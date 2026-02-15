use smash_core::buffer::Buffer;
use smash_syntax::{HighlightEngine, HighlightSpan};

use crate::backend::TerminalBackend;
use crate::cell::Cell;
use crate::error::TuiError;
use crate::pane::Rect;
use crate::screen::Screen;
use crate::style::Style;
use crate::theme::Theme;
use crate::viewport::Viewport;

/// Number of columns reserved for line numbers + separator.
const LINE_NUMBER_WIDTH: u16 = 5; // "1234 "

pub struct Renderer {
    screen: Screen,
    prev_screen: Screen,
}

impl Renderer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            screen: Screen::new(width, height),
            prev_screen: Screen::new(width, height),
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.screen.resize(width, height);
        self.prev_screen.resize(width, height);
    }

    /// Render a buffer with highlights into the given area.
    #[allow(clippy::too_many_arguments)]
    pub fn render_buffer(
        &mut self,
        buffer: &Buffer,
        viewport: &Viewport,
        area: Rect,
        theme: &Theme,
        highlighter: Option<&dyn HighlightEngine>,
        show_line_numbers: bool,
    ) {
        let gutter_w = if show_line_numbers {
            LINE_NUMBER_WIDTH
        } else {
            0
        };
        let text_area_start = area.x + gutter_w;
        let text_area_width = area.width.saturating_sub(gutter_w);
        let line_count = buffer.line_count();

        for screen_row in 0..area.height {
            let buf_line = viewport.top_line() + screen_row as usize;
            let y = area.y + screen_row;

            if buf_line < line_count {
                // Line numbers
                if show_line_numbers {
                    let num_str = format!("{:>4} ", buf_line + 1);
                    let style = theme.line_number_style();
                    for (i, ch) in num_str.chars().enumerate() {
                        let x = area.x + i as u16;
                        if x < area.x + gutter_w {
                            self.screen.set(x, y, Cell::new(ch, style));
                        }
                    }
                }

                // Buffer text
                let line_text = match buffer.line(buf_line) {
                    Some(slice) => slice.to_string(),
                    None => String::new(),
                };
                // Remove trailing newline for display
                let display = line_text.trim_end_matches('\n').trim_end_matches('\r');

                // Get highlights
                let spans: Vec<HighlightSpan> = highlighter
                    .map(|h| h.highlight_line(display))
                    .unwrap_or_default();

                // Render each character
                let left_col = viewport.left_col();
                for (i, ch) in display.chars().enumerate() {
                    if i < left_col {
                        continue;
                    }
                    let col_on_screen = (i - left_col) as u16;
                    if col_on_screen >= text_area_width {
                        break;
                    }
                    let x = text_area_start + col_on_screen;

                    let style = find_style_for_offset(i, &spans, theme);
                    self.screen.set(x, y, Cell::new(ch, style));
                }

                // Clear rest of line
                let chars_written = display.len().saturating_sub(left_col);
                let start = (chars_written as u16).min(text_area_width);
                for col in start..text_area_width {
                    let x = text_area_start + col;
                    self.screen.set(x, y, Cell::new(' ', theme.default_style()));
                }
            } else {
                // Past end of buffer — tilde lines
                if show_line_numbers {
                    let tilde = format!("{:>4} ", "~");
                    let style = theme.line_number_style();
                    for (i, ch) in tilde.chars().enumerate() {
                        let x = area.x + i as u16;
                        if x < area.x + gutter_w {
                            self.screen.set(x, y, Cell::new(ch, style));
                        }
                    }
                }
                for col in 0..text_area_width {
                    let x = text_area_start + col;
                    self.screen.set(x, y, Cell::new(' ', theme.default_style()));
                }
            }
        }
    }

    /// Render status bar at the bottom of the area.
    pub fn render_status_bar(
        &mut self,
        area: Rect,
        filename: &str,
        cursor_line: usize,
        cursor_col: usize,
        modified: bool,
        theme: &Theme,
    ) {
        let y = area.y + area.height.saturating_sub(1);
        let style = theme.status_bar_style();

        let modified_marker = if modified { " [+]" } else { "" };
        let left = format!(" {}{}", filename, modified_marker,);
        let right = format!("{}:{} ", cursor_line + 1, cursor_col + 1,);

        // Fill status bar background
        for col in 0..area.width {
            self.screen.set(area.x + col, y, Cell::new(' ', style));
        }

        // Left side
        for (i, ch) in left.chars().enumerate() {
            if (i as u16) < area.width {
                self.screen.set(area.x + i as u16, y, Cell::new(ch, style));
            }
        }

        // Right side
        let right_start = area.width.saturating_sub(right.len() as u16);
        for (i, ch) in right.chars().enumerate() {
            let x = area.x + right_start + i as u16;
            if x < area.x + area.width {
                self.screen.set(x, y, Cell::new(ch, style));
            }
        }
    }

    /// Flush diff to backend.
    pub fn flush_to_backend(&mut self, backend: &mut dyn TerminalBackend) -> Result<(), TuiError> {
        let diff = self.prev_screen.diff(&self.screen);
        for (col, row, cell) in &diff {
            backend.write_cell(*col, *row, cell)?;
        }
        backend.flush()?;
        self.prev_screen = self.screen.clone();
        Ok(())
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn clear(&mut self) {
        self.screen.clear();
    }
}

fn find_style_for_offset(byte_offset: usize, spans: &[HighlightSpan], theme: &Theme) -> Style {
    for span in spans {
        if byte_offset >= span.start && byte_offset < span.end {
            return theme.scope_style(span.scope);
        }
    }
    theme.default_style()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;
    use crate::theme::default_dark_theme;
    use crate::viewport::Viewport;
    use smash_core::buffer::BufferId;
    use smash_syntax::ScopeId;

    fn make_buffer(text: &str) -> Buffer {
        Buffer::from_text(BufferId::next(), text)
    }

    #[test]
    fn renderer_new_has_correct_dimensions() {
        let r = Renderer::new(80, 24);
        assert_eq!(r.screen().width(), 80);
        assert_eq!(r.screen().height(), 24);
    }

    #[test]
    fn renderer_resize() {
        let mut r = Renderer::new(80, 24);
        r.resize(100, 50);
        assert_eq!(r.screen().width(), 100);
        assert_eq!(r.screen().height(), 50);
    }

    #[test]
    fn renderer_clear() {
        let mut r = Renderer::new(80, 24);
        r.screen.put_char(0, 0, 'X', Style::default());
        r.clear();
        assert_eq!(r.screen().get(0, 0).unwrap().ch, ' ',);
    }

    #[test]
    fn render_buffer_shows_line_numbers() {
        let buf = make_buffer("Hello\nWorld\n");
        let mut r = Renderer::new(80, 24);
        let vp = Viewport::new(24, 75);
        let area = Rect::new(0, 0, 80, 24);
        let theme = default_dark_theme();
        r.render_buffer(&buf, &vp, area, &theme, None, true);
        // Line 1 should show "   1 "
        let c0 = r.screen().get(0, 0).unwrap();
        let c1 = r.screen().get(1, 0).unwrap();
        let c2 = r.screen().get(2, 0).unwrap();
        let c3 = r.screen().get(3, 0).unwrap();
        let c4 = r.screen().get(4, 0).unwrap();
        assert_eq!(c0.ch, ' ');
        assert_eq!(c1.ch, ' ');
        assert_eq!(c2.ch, ' ');
        assert_eq!(c3.ch, '1');
        assert_eq!(c4.ch, ' ');
    }

    #[test]
    fn render_buffer_shows_text() {
        let buf = make_buffer("Hello\n");
        let mut r = Renderer::new(80, 24);
        let vp = Viewport::new(24, 75);
        let area = Rect::new(0, 0, 80, 24);
        let theme = default_dark_theme();
        r.render_buffer(&buf, &vp, area, &theme, None, true);
        // Text starts at col 5 (after line number)
        assert_eq!(r.screen().get(5, 0).unwrap().ch, 'H',);
        assert_eq!(r.screen().get(6, 0).unwrap().ch, 'e',);
        assert_eq!(r.screen().get(7, 0).unwrap().ch, 'l',);
        assert_eq!(r.screen().get(8, 0).unwrap().ch, 'l',);
        assert_eq!(r.screen().get(9, 0).unwrap().ch, 'o',);
    }

    #[test]
    fn render_buffer_without_line_numbers() {
        let buf = make_buffer("Hi\n");
        let mut r = Renderer::new(80, 24);
        let vp = Viewport::new(24, 80);
        let area = Rect::new(0, 0, 80, 24);
        let theme = default_dark_theme();
        r.render_buffer(&buf, &vp, area, &theme, None, false);
        // Text starts at col 0
        assert_eq!(r.screen().get(0, 0).unwrap().ch, 'H',);
        assert_eq!(r.screen().get(1, 0).unwrap().ch, 'i',);
    }

    #[test]
    fn render_buffer_tilde_lines_past_eof() {
        let buf = make_buffer("Line1\n");
        let mut r = Renderer::new(80, 5);
        let vp = Viewport::new(5, 75);
        let area = Rect::new(0, 0, 80, 5);
        let theme = default_dark_theme();
        r.render_buffer(&buf, &vp, area, &theme, None, true);
        // Row 2 (beyond buffer) should show tilde
        // "   ~ " in gutter
        assert_eq!(r.screen().get(3, 2).unwrap().ch, '~',);
    }

    #[test]
    fn render_status_bar_shows_filename_and_position() {
        let mut r = Renderer::new(80, 24);
        let area = Rect::new(0, 0, 80, 24);
        let theme = default_dark_theme();
        r.render_status_bar(area, "test.rs", 10, 5, false, &theme);
        // Status bar is on last row (23)
        let y = 23;
        // Should contain " test.rs"
        assert_eq!(r.screen().get(0, y).unwrap().ch, ' ',);
        assert_eq!(r.screen().get(1, y).unwrap().ch, 't',);
        assert_eq!(r.screen().get(2, y).unwrap().ch, 'e',);
    }

    #[test]
    fn render_status_bar_modified_marker() {
        let mut r = Renderer::new(80, 24);
        let area = Rect::new(0, 0, 80, 24);
        let theme = default_dark_theme();
        r.render_status_bar(area, "test.rs", 0, 0, true, &theme);
        let y = 23;
        // Read the full row to check for [+]
        let mut row_text = String::new();
        for col in 0..80 {
            row_text.push(r.screen().get(col, y).unwrap().ch);
        }
        assert!(row_text.contains("[+]"));
    }

    #[test]
    fn flush_to_backend_writes_diff() {
        let mut r = Renderer::new(10, 5);
        let mut backend = MockBackend::new(10, 5);

        r.screen.put_char(0, 0, 'A', Style::default());
        r.flush_to_backend(&mut backend).unwrap();

        assert_eq!(backend.cell_at(0, 0).unwrap().ch, 'A',);
        assert_eq!(backend.flush_count, 1);
    }

    #[test]
    fn flush_to_backend_no_diff_still_flushes() {
        let mut r = Renderer::new(10, 5);
        let mut backend = MockBackend::new(10, 5);

        // First flush writes everything that differs
        r.flush_to_backend(&mut backend).unwrap();
        // Second flush — no changes
        r.flush_to_backend(&mut backend).unwrap();
        assert_eq!(backend.flush_count, 2);
    }

    #[test]
    fn find_style_for_offset_no_spans_returns_default() {
        let theme = default_dark_theme();
        let style = find_style_for_offset(5, &[], &theme);
        assert_eq!(style, theme.default_style());
    }

    #[test]
    fn find_style_for_offset_matching_span() {
        let theme = default_dark_theme();
        let spans = vec![HighlightSpan::new(0, 5, ScopeId::Keyword)];
        let style = find_style_for_offset(2, &spans, &theme);
        assert_eq!(style, theme.scope_style(ScopeId::Keyword),);
    }

    #[test]
    fn find_style_for_offset_outside_span() {
        let theme = default_dark_theme();
        let spans = vec![HighlightSpan::new(0, 3, ScopeId::Keyword)];
        let style = find_style_for_offset(5, &spans, &theme);
        assert_eq!(style, theme.default_style());
    }

    #[test]
    fn render_buffer_with_viewport_offset() {
        let buf = make_buffer("Line0\nLine1\nLine2\nLine3\nLine4\n");
        let mut r = Renderer::new(80, 3);
        let mut vp = Viewport::new(3, 75);
        vp.set_top_line(2);
        let area = Rect::new(0, 0, 80, 3);
        let theme = default_dark_theme();
        r.render_buffer(&buf, &vp, area, &theme, None, true);
        // Row 0 should show line 3 (buf_line=2)
        // Line number: "   3 "
        assert_eq!(r.screen().get(3, 0).unwrap().ch, '3',);
        // Text "Line2" starts at col 5
        assert_eq!(r.screen().get(5, 0).unwrap().ch, 'L',);
    }
}
