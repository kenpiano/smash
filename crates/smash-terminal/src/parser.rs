use crate::grid::TerminalGrid;

/// Events generated during VT parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// PTY output was processed (informational).
    Output,
    /// Terminal process exited with a status code.
    Exited(i32),
    /// Bell character received.
    Bell,
    /// Title changed via OSC.
    TitleChanged(String),
}

/// Parser states for the VT state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    Csi,
    Osc,
    EscIntermediate,
}

/// A VT escape sequence parser that processes bytes and mutates a `TerminalGrid`.
#[derive(Debug, Clone)]
pub struct VtParser {
    state: ParserState,
    /// Accumulated CSI parameter bytes.
    csi_params: Vec<u8>,
    /// Accumulated CSI intermediate bytes.
    csi_intermediates: Vec<u8>,
    /// Accumulated OSC string.
    osc_string: Vec<u8>,
    /// Accumulated ESC intermediate bytes.
    esc_intermediates: Vec<u8>,
}

impl VtParser {
    /// Create a new parser in the ground state.
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            csi_params: Vec::new(),
            csi_intermediates: Vec::new(),
            osc_string: Vec::new(),
            esc_intermediates: Vec::new(),
        }
    }

    /// Process a chunk of bytes, updating the grid and returning any events.
    pub fn process(&mut self, data: &[u8], grid: &mut TerminalGrid) -> Vec<TerminalEvent> {
        let mut events = Vec::new();

        for &byte in data {
            match self.state {
                ParserState::Ground => {
                    self.handle_ground(byte, grid, &mut events);
                }
                ParserState::Escape => {
                    self.handle_escape(byte, grid, &mut events);
                }
                ParserState::Csi => {
                    self.handle_csi(byte, grid, &mut events);
                }
                ParserState::Osc => {
                    self.handle_osc(byte, grid, &mut events);
                }
                ParserState::EscIntermediate => {
                    self.handle_esc_intermediate(byte, grid, &mut events);
                }
            }
        }

        events
    }

    fn handle_ground(
        &mut self,
        byte: u8,
        grid: &mut TerminalGrid,
        events: &mut Vec<TerminalEvent>,
    ) {
        match byte {
            0x1b => {
                // ESC
                self.state = ParserState::Escape;
                self.esc_intermediates.clear();
            }
            0x07 => {
                // BEL
                events.push(TerminalEvent::Bell);
            }
            0x08 => {
                // BS — backspace
                grid.cursor_left(1);
            }
            0x09 => {
                // HT — tab, advance to next tab stop (every 8 columns)
                let col = grid.cursor.col;
                let next_tab = ((col / 8) + 1) * 8;
                let max_col = grid.size().cols.saturating_sub(1);
                grid.cursor.col = next_tab.min(max_col);
            }
            0x0a..=0x0c => {
                // LF, VT, FF — line feed
                grid.line_feed();
            }
            0x0d => {
                // CR — carriage return
                grid.carriage_return();
            }
            0x20..=0x7e => {
                // Printable ASCII
                grid.write_char(byte as char);
            }
            0xc0..=0xff => {
                // UTF-8 lead byte — simplified: just write replacement char
                // A full implementation would accumulate multi-byte sequences.
                grid.write_char(byte as char);
            }
            _ => {
                // Ignore other control characters
            }
        }
    }

    fn handle_escape(
        &mut self,
        byte: u8,
        grid: &mut TerminalGrid,
        _events: &mut Vec<TerminalEvent>,
    ) {
        match byte {
            b'[' => {
                // CSI introducer
                self.state = ParserState::Csi;
                self.csi_params.clear();
                self.csi_intermediates.clear();
            }
            b']' => {
                // OSC introducer
                self.state = ParserState::Osc;
                self.osc_string.clear();
            }
            b'(' | b')' | b'*' | b'+' => {
                // Designate character set — consume next byte
                self.state = ParserState::EscIntermediate;
                self.esc_intermediates.clear();
                self.esc_intermediates.push(byte);
            }
            b'7' => {
                // Save cursor (DECSC)
                // Just go back to ground for now
                self.state = ParserState::Ground;
            }
            b'8' => {
                // Restore cursor (DECRC)
                self.state = ParserState::Ground;
            }
            b'M' => {
                // Reverse index — move cursor up; if at top of scroll region, scroll down
                if grid.cursor.row == 0 {
                    grid.scroll_down(1);
                } else {
                    grid.cursor_up(1);
                }
                self.state = ParserState::Ground;
            }
            b'D' => {
                // Index — move cursor down; if at bottom of scroll region, scroll up
                grid.line_feed();
                self.state = ParserState::Ground;
            }
            b'E' => {
                // Next line
                grid.carriage_return();
                grid.line_feed();
                self.state = ParserState::Ground;
            }
            b'c' => {
                // Reset terminal (RIS)
                self.state = ParserState::Ground;
            }
            _ => {
                // Unknown ESC sequence, return to ground
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_esc_intermediate(
        &mut self,
        _byte: u8,
        _grid: &mut TerminalGrid,
        _events: &mut Vec<TerminalEvent>,
    ) {
        // Consume the character set designation byte and return to ground
        self.state = ParserState::Ground;
    }

    fn handle_csi(&mut self, byte: u8, grid: &mut TerminalGrid, _events: &mut Vec<TerminalEvent>) {
        match byte {
            b'0'..=b'9' | b';' => {
                // Parameter bytes
                self.csi_params.push(byte);
            }
            b'?' | b'>' | b'!' => {
                // Private mode prefix / intermediate
                self.csi_intermediates.push(byte);
            }
            b' ' | b'"' | b'\'' | b'$' => {
                // Intermediate bytes
                self.csi_intermediates.push(byte);
            }
            0x40..=0x7e => {
                // Final byte — dispatch CSI
                self.dispatch_csi(byte, grid);
                self.state = ParserState::Ground;
            }
            _ => {
                // Invalid, abort
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_osc(&mut self, byte: u8, grid: &mut TerminalGrid, events: &mut Vec<TerminalEvent>) {
        match byte {
            0x07 => {
                // BEL terminates OSC
                self.dispatch_osc(grid, events);
                self.state = ParserState::Ground;
            }
            0x1b => {
                // Could be ST (\x1b\\) — for simplicity, treat ESC as terminator
                // A real parser would check for the following '\\'.
                self.dispatch_osc(grid, events);
                self.state = ParserState::Ground;
            }
            _ => {
                self.osc_string.push(byte);
            }
        }
    }

    fn dispatch_osc(&mut self, grid: &mut TerminalGrid, events: &mut Vec<TerminalEvent>) {
        let osc = String::from_utf8_lossy(&self.osc_string).to_string();

        if let Some(rest) = osc.strip_prefix("0;").or_else(|| osc.strip_prefix("2;")) {
            // Set title
            let title = rest.to_string();
            grid.title = Some(title.clone());
            events.push(TerminalEvent::TitleChanged(title));
        } else if let Some(rest) = osc.strip_prefix("8;") {
            // OSC 8 — hyperlink
            // Format: 8;params;uri
            let parts: Vec<&str> = rest.splitn(2, ';').collect();
            if parts.len() == 2 {
                let uri = parts[1];
                if uri.is_empty() {
                    // Close hyperlink — handled at grid level
                } else {
                    // Store for subsequent characters (simplified)
                    tracing::debug!("OSC 8 hyperlink: {}", uri);
                }
            }
        }

        self.osc_string.clear();
    }

    /// Parse CSI parameter string into a list of numbers.
    fn parse_params(&self) -> Vec<u16> {
        if self.csi_params.is_empty() {
            return Vec::new();
        }

        let s = String::from_utf8_lossy(&self.csi_params);
        s.split(';')
            .map(|p| {
                if p.is_empty() {
                    0
                } else {
                    p.parse::<u16>().unwrap_or(0)
                }
            })
            .collect()
    }

    fn dispatch_csi(&mut self, final_byte: u8, grid: &mut TerminalGrid) {
        let params = self.parse_params();
        let is_private = self.csi_intermediates.contains(&b'?');

        match final_byte {
            b'A' => {
                // CUU — Cursor Up
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.cursor_up(n);
            }
            b'B' => {
                // CUD — Cursor Down
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.cursor_down(n);
            }
            b'C' => {
                // CUF — Cursor Forward (right)
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.cursor_right(n);
            }
            b'D' => {
                // CUB — Cursor Back (left)
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.cursor_left(n);
            }
            b'H' | b'f' => {
                // CUP — Cursor Position (1-based in params, convert to 0-based)
                let row = params.first().copied().unwrap_or(1).max(1) - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) - 1;
                grid.cursor_set_position(row, col);
            }
            b'J' => {
                // ED — Erase Display
                let mode = params.first().copied().unwrap_or(0);
                grid.erase_display_variant(mode);
            }
            b'K' => {
                // EL — Erase Line
                let mode = params.first().copied().unwrap_or(0);
                grid.erase_line_variant(mode);
            }
            b'S' => {
                // SU — Scroll Up
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.scroll_up(n);
            }
            b'T' => {
                // SD — Scroll Down
                let n = params.first().copied().unwrap_or(1).max(1);
                grid.scroll_down(n);
            }
            b'r' => {
                // DECSTBM — Set Scrolling Region (1-based)
                let top = params.first().copied().unwrap_or(1).max(1) - 1;
                let bottom = params.get(1).copied().unwrap_or(grid.size().rows).max(1) - 1;
                grid.set_scroll_region(top, bottom);
                grid.cursor_set_position(0, 0);
            }
            b'm' => {
                // SGR — Select Graphic Rendition
                self.dispatch_sgr(&params, grid);
            }
            b'h' if is_private => {
                // DECSET — Private mode set
                for &p in &params {
                    match p {
                        1049 => {
                            // Alternate screen buffer
                            grid.enter_alternate_screen();
                        }
                        25 => {
                            // Show cursor (ignored for now)
                        }
                        _ => {}
                    }
                }
            }
            b'l' if is_private => {
                // DECRST — Private mode reset
                for &p in &params {
                    match p {
                        1049 => {
                            grid.leave_alternate_screen();
                        }
                        25 => {
                            // Hide cursor (ignored for now)
                        }
                        _ => {}
                    }
                }
            }
            b'd' => {
                // VPA — Vertical Position Absolute (1-based)
                let row = params.first().copied().unwrap_or(1).max(1) - 1;
                grid.cursor.row = row.min(grid.size().rows.saturating_sub(1));
            }
            b'G' | b'`' => {
                // CHA — Cursor Character Absolute (1-based)
                let col = params.first().copied().unwrap_or(1).max(1) - 1;
                grid.cursor.col = col.min(grid.size().cols.saturating_sub(1));
            }
            b'L' => {
                // IL — Insert Lines
                let _n = params.first().copied().unwrap_or(1).max(1);
                // Simplified: just scroll down within region
            }
            b'M' => {
                // DL — Delete Lines
                let _n = params.first().copied().unwrap_or(1).max(1);
            }
            b'@' => {
                // ICH — Insert Characters (blank)
                let _n = params.first().copied().unwrap_or(1).max(1);
            }
            b'P' => {
                // DCH — Delete Characters
                let _n = params.first().copied().unwrap_or(1).max(1);
            }
            _ => {
                tracing::trace!("unhandled CSI final byte: {:?}", final_byte as char);
            }
        }
    }

    fn dispatch_sgr(&self, params: &[u16], grid: &mut TerminalGrid) {
        if params.is_empty() {
            // No params = reset
            self.apply_sgr_code(0, grid);
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let code = params[i];
            match code {
                38 => {
                    // Set foreground color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                // 256 color: 38;5;n
                                if i + 2 < params.len() {
                                    let idx = params[i + 2] as u8;
                                    grid.current_fg = crate::grid::Color::Indexed(idx);
                                    i += 3;
                                    continue;
                                }
                            }
                            2 => {
                                // RGB: 38;2;r;g;b
                                if i + 4 < params.len() {
                                    let r = params[i + 2] as u8;
                                    let g = params[i + 3] as u8;
                                    let b = params[i + 4] as u8;
                                    grid.current_fg = crate::grid::Color::Rgb(r, g, b);
                                    i += 5;
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }
                48 => {
                    // Set background color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                if i + 2 < params.len() {
                                    let idx = params[i + 2] as u8;
                                    grid.current_bg = crate::grid::Color::Indexed(idx);
                                    i += 3;
                                    continue;
                                }
                            }
                            2 => {
                                if i + 4 < params.len() {
                                    let r = params[i + 2] as u8;
                                    let g = params[i + 3] as u8;
                                    let b = params[i + 4] as u8;
                                    grid.current_bg = crate::grid::Color::Rgb(r, g, b);
                                    i += 5;
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }
                _ => {
                    self.apply_sgr_code(code, grid);
                    i += 1;
                }
            }
        }
    }

    fn apply_sgr_code(&self, code: u16, grid: &mut TerminalGrid) {
        match code {
            0 => {
                // Reset all
                grid.current_attrs = crate::grid::CellAttributes::default();
                grid.current_fg = crate::grid::Color::Default;
                grid.current_bg = crate::grid::Color::Default;
            }
            1 => grid.current_attrs.bold = true,
            2 => grid.current_attrs.dim = true,
            3 => grid.current_attrs.italic = true,
            4 => grid.current_attrs.underline = true,
            7 => grid.current_attrs.reverse = true,
            8 => grid.current_attrs.hidden = true,
            9 => grid.current_attrs.strikethrough = true,
            22 => {
                grid.current_attrs.bold = false;
                grid.current_attrs.dim = false;
            }
            23 => grid.current_attrs.italic = false,
            24 => grid.current_attrs.underline = false,
            27 => grid.current_attrs.reverse = false,
            28 => grid.current_attrs.hidden = false,
            29 => grid.current_attrs.strikethrough = false,
            // Standard foreground colors 30-37
            30..=37 => {
                grid.current_fg = crate::grid::Color::Indexed(code as u8 - 30);
            }
            39 => grid.current_fg = crate::grid::Color::Default,
            // Standard background colors 40-47
            40..=47 => {
                grid.current_bg = crate::grid::Color::Indexed(code as u8 - 40);
            }
            49 => grid.current_bg = crate::grid::Color::Default,
            // Bright foreground colors 90-97
            90..=97 => {
                grid.current_fg = crate::grid::Color::Indexed(code as u8 - 90 + 8);
            }
            // Bright background colors 100-107
            100..=107 => {
                grid.current_bg = crate::grid::Color::Indexed(code as u8 - 100 + 8);
            }
            _ => {
                tracing::trace!("unhandled SGR code: {}", code);
            }
        }
    }
}

impl Default for VtParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{Color, TerminalGrid};

    #[test]
    fn parser_plain_text() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"hello", &mut grid);

        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'h');
        assert_eq!(grid.get_cell(0, 1).unwrap().character, 'e');
        assert_eq!(grid.get_cell(0, 2).unwrap().character, 'l');
        assert_eq!(grid.get_cell(0, 3).unwrap().character, 'l');
        assert_eq!(grid.get_cell(0, 4).unwrap().character, 'o');
    }

    #[test]
    fn parser_csi_cursor_up() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Move cursor to row 5 first
        grid.cursor_set_position(5, 0);
        // CSI A — cursor up 1
        parser.process(b"\x1b[A", &mut grid);
        assert_eq!(grid.cursor.row, 4);
    }

    #[test]
    fn parser_csi_cursor_position() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 5;10 H — cursor to row 5, col 10 (1-based → 4, 9 0-based)
        parser.process(b"\x1b[5;10H", &mut grid);
        assert_eq!(grid.cursor.row, 4);
        assert_eq!(grid.cursor.col, 9);
    }

    #[test]
    fn parser_sgr_bold() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // SGR 1 — bold
        parser.process(b"\x1b[1m", &mut grid);
        assert!(grid.current_attrs.bold);

        // Write a character — it should have bold attribute
        parser.process(b"X", &mut grid);
        let cell = grid.get_cell(0, 0).unwrap();
        assert_eq!(cell.character, 'X');
        assert!(cell.attrs.bold);
    }

    #[test]
    fn parser_sgr_fg_color() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // SGR 38;5;196 — indexed fg color 196
        parser.process(b"\x1b[38;5;196m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(196));

        parser.process(b"A", &mut grid);
        let cell = grid.get_cell(0, 0).unwrap();
        assert_eq!(cell.fg, Color::Indexed(196));
    }

    #[test]
    fn parser_sgr_rgb_color() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // SGR 38;2;255;0;0 — RGB red foreground
        parser.process(b"\x1b[38;2;255;0;0m", &mut grid);
        assert_eq!(grid.current_fg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn parser_sgr_reset() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Set bold and a color
        parser.process(b"\x1b[1;38;5;196m", &mut grid);
        assert!(grid.current_attrs.bold);
        assert_eq!(grid.current_fg, Color::Indexed(196));

        // Reset
        parser.process(b"\x1b[0m", &mut grid);
        assert!(!grid.current_attrs.bold);
        assert_eq!(grid.current_fg, Color::Default);
        assert_eq!(grid.current_bg, Color::Default);
    }

    #[test]
    fn parser_osc_set_title() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // OSC 0 — set title, terminated by BEL
        let events = parser.process(b"\x1b]0;My Title\x07", &mut grid);

        assert_eq!(grid.title, Some("My Title".to_string()));
        assert!(events.contains(&TerminalEvent::TitleChanged("My Title".to_string())));
    }

    #[test]
    fn parser_scroll_region() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // DECSTBM: set scroll region to rows 5–20 (1-based)
        parser.process(b"\x1b[5;20r", &mut grid);

        // Cursor should be moved to home position after setting scroll region
        assert_eq!(grid.cursor.row, 0);
        assert_eq!(grid.cursor.col, 0);
    }

    #[test]
    fn parser_alternate_screen() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();

        // Write on primary
        parser.process(b"Primary", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'P');

        // Enter alternate screen
        parser.process(b"\x1b[?1049h", &mut grid);
        assert!(grid.is_alternate_screen());
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');

        // Write on alternate
        parser.process(b"Alt", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');

        // Leave alternate screen
        parser.process(b"\x1b[?1049l", &mut grid);
        assert!(!grid.is_alternate_screen());
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'P');
    }

    #[test]
    fn parser_bell_event() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        let events = parser.process(b"\x07", &mut grid);
        assert!(events.contains(&TerminalEvent::Bell));
    }

    #[test]
    fn parser_carriage_return_line_feed() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Hello\r\nWorld", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'H');
        assert_eq!(grid.get_cell(1, 0).unwrap().character, 'W');
    }

    #[test]
    fn parser_backspace() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"AB\x08C", &mut grid);
        // 'A' at col 0, then 'B' at col 1, backspace to col 1, 'C' overwrites 'B'
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(0, 1).unwrap().character, 'C');
    }

    #[test]
    fn parser_erase_display() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Hello", &mut grid);
        parser.process(b"\x1b[2J", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');
    }

    #[test]
    fn parser_erase_line() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Hello", &mut grid);
        grid.cursor_set_position(0, 0);
        parser.process(b"\x1b[2K", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');
        assert_eq!(grid.get_cell(0, 4).unwrap().character, ' ');
    }

    #[test]
    fn parser_cursor_movement_sequences() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();

        // Move to 5,5
        grid.cursor_set_position(5, 5);

        // Cursor down 3
        parser.process(b"\x1b[3B", &mut grid);
        assert_eq!(grid.cursor.row, 8);

        // Cursor right 2
        parser.process(b"\x1b[2C", &mut grid);
        assert_eq!(grid.cursor.col, 7);

        // Cursor left 1
        parser.process(b"\x1b[1D", &mut grid);
        assert_eq!(grid.cursor.col, 6);
    }

    #[test]
    fn parser_sgr_multiple_attrs() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Bold + italic + underline in one sequence
        parser.process(b"\x1b[1;3;4m", &mut grid);
        assert!(grid.current_attrs.bold);
        assert!(grid.current_attrs.italic);
        assert!(grid.current_attrs.underline);
    }

    #[test]
    fn parser_tab_advance() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"A\tB", &mut grid);
        // 'A' at col 0, tab to col 8, 'B' at col 8
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(0, 8).unwrap().character, 'B');
    }

    // --- SGR attribute edge cases ---

    #[test]
    fn parser_sgr_dim_attribute() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[2m", &mut grid);
        assert!(grid.current_attrs.dim);
    }

    #[test]
    fn parser_sgr_italic_attribute() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[3m", &mut grid);
        assert!(grid.current_attrs.italic);
    }

    #[test]
    fn parser_sgr_reverse_attribute() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[7m", &mut grid);
        assert!(grid.current_attrs.reverse);
    }

    #[test]
    fn parser_sgr_hidden_attribute() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[8m", &mut grid);
        assert!(grid.current_attrs.hidden);
    }

    #[test]
    fn parser_sgr_strikethrough_attribute() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[9m", &mut grid);
        assert!(grid.current_attrs.strikethrough);
    }

    #[test]
    fn parser_sgr_reset_bold_dim_code22() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[1;2m", &mut grid);
        assert!(grid.current_attrs.bold);
        assert!(grid.current_attrs.dim);
        parser.process(b"\x1b[22m", &mut grid);
        assert!(!grid.current_attrs.bold);
        assert!(!grid.current_attrs.dim);
    }

    #[test]
    fn parser_sgr_reset_italic_code23() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[3m", &mut grid);
        assert!(grid.current_attrs.italic);
        parser.process(b"\x1b[23m", &mut grid);
        assert!(!grid.current_attrs.italic);
    }

    #[test]
    fn parser_sgr_reset_underline_code24() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[4m", &mut grid);
        assert!(grid.current_attrs.underline);
        parser.process(b"\x1b[24m", &mut grid);
        assert!(!grid.current_attrs.underline);
    }

    #[test]
    fn parser_sgr_reset_reverse_code27() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[7m", &mut grid);
        assert!(grid.current_attrs.reverse);
        parser.process(b"\x1b[27m", &mut grid);
        assert!(!grid.current_attrs.reverse);
    }

    #[test]
    fn parser_sgr_reset_hidden_code28() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[8m", &mut grid);
        assert!(grid.current_attrs.hidden);
        parser.process(b"\x1b[28m", &mut grid);
        assert!(!grid.current_attrs.hidden);
    }

    #[test]
    fn parser_sgr_reset_strikethrough_code29() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[9m", &mut grid);
        assert!(grid.current_attrs.strikethrough);
        parser.process(b"\x1b[29m", &mut grid);
        assert!(!grid.current_attrs.strikethrough);
    }

    // --- SGR color codes ---

    #[test]
    fn parser_sgr_standard_fg_colors() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Set red foreground (31)
        parser.process(b"\x1b[31m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(1));
        // Set green foreground (32)
        parser.process(b"\x1b[32m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(2));
    }

    #[test]
    fn parser_sgr_default_fg_color_39() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[31m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(1));
        parser.process(b"\x1b[39m", &mut grid);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_standard_bg_colors() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Set red background (41)
        parser.process(b"\x1b[41m", &mut grid);
        assert_eq!(grid.current_bg, Color::Indexed(1));
    }

    #[test]
    fn parser_sgr_default_bg_color_49() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[41m", &mut grid);
        assert_eq!(grid.current_bg, Color::Indexed(1));
        parser.process(b"\x1b[49m", &mut grid);
        assert_eq!(grid.current_bg, Color::Default);
    }

    #[test]
    fn parser_sgr_bright_fg_colors() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Bright red foreground (91)
        parser.process(b"\x1b[91m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(9));
        // Bright white foreground (97)
        parser.process(b"\x1b[97m", &mut grid);
        assert_eq!(grid.current_fg, Color::Indexed(15));
    }

    #[test]
    fn parser_sgr_bright_bg_colors() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Bright red background (101)
        parser.process(b"\x1b[101m", &mut grid);
        assert_eq!(grid.current_bg, Color::Indexed(9));
    }

    #[test]
    fn parser_sgr_bg_256_color() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[48;5;42m", &mut grid);
        assert_eq!(grid.current_bg, Color::Indexed(42));
    }

    #[test]
    fn parser_sgr_bg_rgb_color() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[48;2;10;20;30m", &mut grid);
        assert_eq!(grid.current_bg, Color::Rgb(10, 20, 30));
    }

    #[test]
    fn parser_sgr_empty_params_resets() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[1m", &mut grid);
        assert!(grid.current_attrs.bold);
        // ESC[m with no params should reset
        parser.process(b"\x1b[m", &mut grid);
        assert!(!grid.current_attrs.bold);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_38_unknown_submode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // 38;9 is not a valid sub-mode (not 2 or 5), should just advance
        parser.process(b"\x1b[38;9m", &mut grid);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_48_unknown_submode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[48;9m", &mut grid);
        assert_eq!(grid.current_bg, Color::Default);
    }

    #[test]
    fn parser_sgr_38_5_insufficient_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // 38;5 without the color index
        parser.process(b"\x1b[38;5m", &mut grid);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_38_2_insufficient_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // 38;2;r;g without 'b'
        parser.process(b"\x1b[38;2;10;20m", &mut grid);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_48_5_insufficient_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[48;5m", &mut grid);
        assert_eq!(grid.current_bg, Color::Default);
    }

    #[test]
    fn parser_sgr_48_2_insufficient_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[48;2;1;2m", &mut grid);
        assert_eq!(grid.current_bg, Color::Default);
    }

    #[test]
    fn parser_sgr_unhandled_code() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Code 99 is unhandled, should not panic
        parser.process(b"\x1b[99m", &mut grid);
    }

    // --- CSI erase operations ---

    #[test]
    fn parser_erase_display_mode0_from_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"ABCDE", &mut grid);
        grid.cursor_set_position(0, 2);
        // ED mode 0: erase from cursor to end
        parser.process(b"\x1b[0J", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(0, 1).unwrap().character, 'B');
    }

    #[test]
    fn parser_erase_display_mode1_to_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"ABCDE", &mut grid);
        grid.cursor_set_position(0, 2);
        // ED mode 1: erase from start to cursor
        parser.process(b"\x1b[1J", &mut grid);
    }

    #[test]
    fn parser_erase_line_mode0() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"ABCDE", &mut grid);
        grid.cursor_set_position(0, 2);
        // EL mode 0: erase from cursor to end of line
        parser.process(b"\x1b[0K", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(0, 1).unwrap().character, 'B');
    }

    #[test]
    fn parser_erase_line_mode1() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"ABCDE", &mut grid);
        grid.cursor_set_position(0, 2);
        // EL mode 1: erase from start of line to cursor
        parser.process(b"\x1b[1K", &mut grid);
    }

    // --- CSI cursor position with 'f' final byte ---

    #[test]
    fn parser_csi_cursor_position_f() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 3;7 f — same as H
        parser.process(b"\x1b[3;7f", &mut grid);
        assert_eq!(grid.cursor.row, 2);
        assert_eq!(grid.cursor.col, 6);
    }

    // --- CSI VPA and CHA ---

    #[test]
    fn parser_csi_vpa() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 10 d — VPA, move to row 10 (1-based → row 9)
        parser.process(b"\x1b[10d", &mut grid);
        assert_eq!(grid.cursor.row, 9);
    }

    #[test]
    fn parser_csi_cha_g() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 15 G — CHA, column 15 (1-based → col 14)
        parser.process(b"\x1b[15G", &mut grid);
        assert_eq!(grid.cursor.col, 14);
    }

    #[test]
    fn parser_csi_cha_backtick() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 5 ` — same as CHA
        parser.process(b"\x1b[5`", &mut grid);
        assert_eq!(grid.cursor.col, 4);
    }

    // --- CSI scroll ---

    #[test]
    fn parser_csi_scroll_up() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Line1", &mut grid);
        // CSI 1 S — scroll up 1 line
        parser.process(b"\x1b[1S", &mut grid);
    }

    #[test]
    fn parser_csi_scroll_down() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Line1", &mut grid);
        // CSI 1 T — scroll down 1 line
        parser.process(b"\x1b[1T", &mut grid);
    }

    // --- CSI insert/delete lines and characters ---

    #[test]
    fn parser_csi_insert_lines() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 2 L — insert 2 lines
        parser.process(b"\x1b[2L", &mut grid);
    }

    #[test]
    fn parser_csi_delete_lines() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 3 M — delete 3 lines
        parser.process(b"\x1b[3M", &mut grid);
    }

    #[test]
    fn parser_csi_insert_characters() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 2 @ — insert 2 blank characters
        parser.process(b"\x1b[2@", &mut grid);
    }

    #[test]
    fn parser_csi_delete_characters() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI 2 P — delete 2 characters
        parser.process(b"\x1b[2P", &mut grid);
    }

    // --- Cursor save/restore ---

    #[test]
    fn parser_esc_save_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        grid.cursor_set_position(5, 10);
        // ESC 7 — save cursor
        parser.process(b"\x1b7", &mut grid);
        // Parser should return to ground state
        parser.process(b"A", &mut grid);
        assert_eq!(grid.get_cell(5, 10).unwrap().character, 'A');
    }

    #[test]
    fn parser_esc_restore_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // ESC 8 — restore cursor
        parser.process(b"\x1b8", &mut grid);
        // Should return to ground
        parser.process(b"B", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'B');
    }

    // --- ESC sequences ---

    #[test]
    fn parser_esc_reverse_index_at_top() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Cursor at row 0, ESC M should scroll down
        grid.cursor_set_position(0, 0);
        parser.process(b"\x1bM", &mut grid);
        assert_eq!(grid.cursor.row, 0);
    }

    #[test]
    fn parser_esc_reverse_index_not_at_top() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        grid.cursor_set_position(3, 0);
        // ESC M — cursor up
        parser.process(b"\x1bM", &mut grid);
        assert_eq!(grid.cursor.row, 2);
    }

    #[test]
    fn parser_esc_index_d() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // ESC D — index (line feed)
        parser.process(b"\x1bD", &mut grid);
        assert_eq!(grid.cursor.row, 1);
    }

    #[test]
    fn parser_esc_next_line() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        grid.cursor.col = 5;
        // ESC E — next line (CR + LF)
        parser.process(b"\x1bE", &mut grid);
        assert_eq!(grid.cursor.row, 1);
        assert_eq!(grid.cursor.col, 0);
    }

    #[test]
    fn parser_esc_reset_terminal() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // ESC c — RIS (terminal reset)
        parser.process(b"\x1bc", &mut grid);
        // Should return to ground; can still write
        parser.process(b"X", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'X');
    }

    #[test]
    fn parser_esc_unknown_sequence() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // ESC Z — unknown, should return to ground
        parser.process(b"\x1bZ", &mut grid);
        parser.process(b"A", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
    }

    #[test]
    fn parser_esc_charset_designation() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // ESC ( B — designate US ASCII charset; the 'B' is consumed by EscIntermediate
        parser.process(b"\x1b(B", &mut grid);
        // Should be back in ground
        parser.process(b"A", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
    }

    // --- OSC edge cases ---

    #[test]
    fn parser_osc_title_prefix2() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // OSC 2; sets title via prefix "2;"
        let events = parser.process(b"\x1b]2;Window Title\x07", &mut grid);
        assert_eq!(grid.title, Some("Window Title".to_string()));
        assert!(events.contains(&TerminalEvent::TitleChanged("Window Title".to_string())));
    }

    #[test]
    fn parser_osc_esc_terminator() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // OSC terminated by ESC (ST simplified)
        let events = parser.process(b"\x1b]0;ESC Title\x1b", &mut grid);
        assert_eq!(grid.title, Some("ESC Title".to_string()));
        assert!(events.contains(&TerminalEvent::TitleChanged("ESC Title".to_string())));
    }

    #[test]
    fn parser_osc_hyperlink() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // OSC 8 hyperlink with URI
        parser.process(b"\x1b]8;;https://example.com\x07", &mut grid);
        // Close hyperlink
        parser.process(b"\x1b]8;;\x07", &mut grid);
    }

    #[test]
    fn parser_osc_unknown() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Unknown OSC code
        parser.process(b"\x1b]999;data\x07", &mut grid);
    }

    // --- Ground state edge cases ---

    #[test]
    fn parser_vt_and_ff_as_linefeed() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // VT (0x0b) and FF (0x0c) should act as line feed
        parser.process(b"A\x0bB\x0cC", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(1, 1).unwrap().character, 'B');
    }

    #[test]
    fn parser_utf8_lead_byte() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // UTF-8 lead byte (>= 0xc0) — simplified handling writes as char
        parser.process(&[0xc3], &mut grid);
        // Should not panic and cursor should advance
    }

    #[test]
    fn parser_ignored_control_chars() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Various control characters that should be ignored (e.g., NUL, SOH)
        parser.process(&[0x00, 0x01, 0x02], &mut grid);
        // Cursor should not have moved, no crash
        assert_eq!(grid.cursor.col, 0);
    }

    // --- CSI edge cases ---

    #[test]
    fn parser_csi_intermediate_bytes() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI with intermediate bytes like space (for DECSCUSR etc.)
        parser.process(b"\x1b[2 q", &mut grid);
    }

    #[test]
    fn parser_csi_private_mode_prefix_gt() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI > c — DA2
        parser.process(b"\x1b[>c", &mut grid);
    }

    #[test]
    fn parser_csi_private_mode_prefix_bang() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI ! p — soft terminal reset
        parser.process(b"\x1b[!p", &mut grid);
    }

    #[test]
    fn parser_csi_invalid_byte_aborts() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Invalid byte in CSI should abort and return to ground
        parser.process(b"\x1b[\x01A", &mut grid);
        // Should be back in ground — 'A' should be written as printable
        // (The \x01 aborts CSI, then 'A' is ignored since handle_csi already consumed it,
        // but we just verify no panic)
    }

    #[test]
    fn parser_csi_unknown_final_byte() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI with unhandled final byte 'z'
        parser.process(b"\x1b[1z", &mut grid);
        // Should return to ground
        parser.process(b"X", &mut grid);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'X');
    }

    // --- DECSET/DECRST edge cases ---

    #[test]
    fn parser_decset_show_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // DECSET ?25h — show cursor
        parser.process(b"\x1b[?25h", &mut grid);
    }

    #[test]
    fn parser_decrst_hide_cursor() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // DECRST ?25l — hide cursor
        parser.process(b"\x1b[?25l", &mut grid);
    }

    #[test]
    fn parser_decset_unknown_mode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Unknown private mode
        parser.process(b"\x1b[?9999h", &mut grid);
    }

    #[test]
    fn parser_decrst_unknown_mode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[?9999l", &mut grid);
    }

    // --- Default trait ---

    #[test]
    fn parser_default_trait() {
        let parser = VtParser::default();
        assert_eq!(parser.state, ParserState::Ground);
    }

    // --- CSI H with default params ---

    #[test]
    fn parser_csi_cursor_home_no_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        grid.cursor_set_position(5, 10);
        // CSI H with no params — should go to 0,0
        parser.process(b"\x1b[H", &mut grid);
        assert_eq!(grid.cursor.row, 0);
        assert_eq!(grid.cursor.col, 0);
    }

    // --- SGR 38 with no following params ---

    #[test]
    fn parser_sgr_38_alone() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // 38 alone without sub-params
        parser.process(b"\x1b[38m", &mut grid);
        assert_eq!(grid.current_fg, Color::Default);
    }

    #[test]
    fn parser_sgr_48_alone() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // 48 alone without sub-params
        parser.process(b"\x1b[48m", &mut grid);
        assert_eq!(grid.current_bg, Color::Default);
    }

    // --- CSI erase with default (no explicit param) ---

    #[test]
    fn parser_erase_display_default_mode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Hello", &mut grid);
        grid.cursor_set_position(0, 2);
        // ESC[J with no param defaults to mode 0
        parser.process(b"\x1b[J", &mut grid);
    }

    #[test]
    fn parser_erase_line_default_mode() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"Hello", &mut grid);
        grid.cursor_set_position(0, 2);
        // ESC[K with no param defaults to mode 0
        parser.process(b"\x1b[K", &mut grid);
    }

    // --- CSI intermediate: quote, apostrophe, dollar ---

    #[test]
    fn parser_csi_intermediate_quote() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI with " intermediate
        parser.process(b"\x1b[0\"p", &mut grid);
    }

    #[test]
    fn parser_csi_intermediate_apostrophe() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI with ' intermediate
        parser.process(b"\x1b[0'z", &mut grid);
    }

    #[test]
    fn parser_csi_intermediate_dollar() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // CSI with $ intermediate
        parser.process(b"\x1b[0$p", &mut grid);
    }

    // --- Multiple DECSET/DECRST params ---

    #[test]
    fn parser_decset_multiple_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        // Set both alt screen (1049) and show cursor (25) in one sequence
        parser.process(b"\x1b[?1049;25h", &mut grid);
        assert!(grid.is_alternate_screen());
    }

    #[test]
    fn parser_decrst_multiple_params() {
        let mut grid = TerminalGrid::new(80, 24);
        let mut parser = VtParser::new();
        parser.process(b"\x1b[?1049h", &mut grid);
        // Reset both
        parser.process(b"\x1b[?1049;25l", &mut grid);
        assert!(!grid.is_alternate_screen());
    }
}
