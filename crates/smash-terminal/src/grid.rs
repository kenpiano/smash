use serde::{Deserialize, Serialize};

/// A color in the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Color {
    /// Default terminal color.
    #[default]
    Default,
    /// 256-color palette index.
    Indexed(u8),
    /// True-color RGB.
    Rgb(u8, u8, u8),
}

/// Cell display attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellAttributes {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
    pub hidden: bool,
}

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttributes,
    /// Optional hyperlink URL associated with this cell.
    pub hyperlink: Option<String>,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttributes::default(),
            hyperlink: None,
        }
    }
}

/// Terminal dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Cursor position within the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
}

/// The 2D terminal grid maintaining cell contents, cursor, and scroll state.
#[derive(Debug, Clone)]
pub struct TerminalGrid {
    size: TerminalSize,
    /// Primary screen buffer.
    primary: Vec<Vec<TerminalCell>>,
    /// Alternate screen buffer.
    alternate: Vec<Vec<TerminalCell>>,
    /// Whether the alternate screen is active.
    using_alternate: bool,
    /// Scrollback buffer (lines scrolled off the top of primary screen).
    scrollback: Vec<Vec<TerminalCell>>,
    /// Maximum scrollback size.
    scrollback_limit: usize,
    /// Current cursor position.
    pub cursor: CursorPosition,
    /// Saved cursor for alternate screen switching.
    saved_cursor: CursorPosition,
    /// Scroll region (top, bottom), inclusive, 0-based.
    scroll_region_top: u16,
    scroll_region_bottom: u16,
    /// Current default cell attributes for new text.
    pub current_attrs: CellAttributes,
    /// Current foreground color for new text.
    pub current_fg: Color,
    /// Current background color for new text.
    pub current_bg: Color,
    /// Title set by OSC sequences.
    pub title: Option<String>,
}

impl TerminalGrid {
    /// Create a new grid with the given column and row count.
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = TerminalSize { cols, rows };
        let primary = Self::create_buffer(cols, rows);
        let alternate = Self::create_buffer(cols, rows);
        Self {
            size,
            primary,
            alternate,
            using_alternate: false,
            scrollback: Vec::new(),
            scrollback_limit: 10_000,
            cursor: CursorPosition::default(),
            saved_cursor: CursorPosition::default(),
            scroll_region_top: 0,
            scroll_region_bottom: rows.saturating_sub(1),
            current_attrs: CellAttributes::default(),
            current_fg: Color::Default,
            current_bg: Color::Default,
            title: None,
        }
    }

    fn create_buffer(cols: u16, rows: u16) -> Vec<Vec<TerminalCell>> {
        (0..rows as usize)
            .map(|_| {
                (0..cols as usize)
                    .map(|_| TerminalCell::default())
                    .collect()
            })
            .collect()
    }

    /// Get the current size.
    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// Get a reference to the active screen buffer.
    fn active_buffer(&self) -> &Vec<Vec<TerminalCell>> {
        if self.using_alternate {
            &self.alternate
        } else {
            &self.primary
        }
    }

    /// Get a mutable reference to the active screen buffer.
    fn active_buffer_mut(&mut self) -> &mut Vec<Vec<TerminalCell>> {
        if self.using_alternate {
            &mut self.alternate
        } else {
            &mut self.primary
        }
    }

    /// Get a cell at the given row and column.
    pub fn get_cell(&self, row: u16, col: u16) -> Option<&TerminalCell> {
        let buf = self.active_buffer();
        buf.get(row as usize).and_then(|r| r.get(col as usize))
    }

    /// Set a cell at the given row and column.
    pub fn set_cell(&mut self, row: u16, col: u16, cell: TerminalCell) {
        let rows = self.size.rows;
        let cols = self.size.cols;
        if row < rows && col < cols {
            let buf = self.active_buffer_mut();
            buf[row as usize][col as usize] = cell;
        }
    }

    /// Write a character at the current cursor position using current attributes,
    /// then advance the cursor.
    pub fn write_char(&mut self, ch: char) {
        // If cursor is at the right edge, wrap to next line
        if self.cursor.col >= self.size.cols {
            self.cursor.col = 0;
            self.cursor_down_with_scroll();
        }

        let cell = TerminalCell {
            character: ch,
            fg: self.current_fg,
            bg: self.current_bg,
            attrs: self.current_attrs,
            hyperlink: None,
        };

        let row = self.cursor.row;
        let col = self.cursor.col;
        self.set_cell(row, col, cell);
        self.cursor.col += 1;
    }

    /// Move cursor up by n rows, clamping at the top of the screen.
    pub fn cursor_up(&mut self, n: u16) {
        self.cursor.row = self.cursor.row.saturating_sub(n);
    }

    /// Move cursor down by n rows, clamping at the bottom of the screen.
    pub fn cursor_down(&mut self, n: u16) {
        self.cursor.row = (self.cursor.row + n).min(self.size.rows.saturating_sub(1));
    }

    /// Move cursor left by n columns, clamping at column 0.
    pub fn cursor_left(&mut self, n: u16) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Move cursor right by n columns, clamping at the right edge.
    pub fn cursor_right(&mut self, n: u16) {
        self.cursor.col = (self.cursor.col + n).min(self.size.cols.saturating_sub(1));
    }

    /// Set cursor to an absolute position (0-based).
    pub fn cursor_set_position(&mut self, row: u16, col: u16) {
        self.cursor.row = row.min(self.size.rows.saturating_sub(1));
        self.cursor.col = col.min(self.size.cols.saturating_sub(1));
    }

    /// Move cursor down by one row, scrolling the region if at the bottom.
    fn cursor_down_with_scroll(&mut self) {
        if self.cursor.row >= self.scroll_region_bottom {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
    }

    /// Perform a carriage return (move cursor to column 0).
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    /// Perform a line feed (move cursor down, scroll if needed).
    pub fn line_feed(&mut self) {
        self.cursor_down_with_scroll();
    }

    /// Scroll the scroll region up by n lines.
    /// The top line(s) of the scroll region are removed and blank lines
    /// are inserted at the bottom of the scroll region.
    pub fn scroll_up(&mut self, n: u16) {
        let top = self.scroll_region_top as usize;
        let bottom = self.scroll_region_bottom as usize;
        let cols = self.size.cols;

        for _ in 0..n {
            if top <= bottom {
                let buf = self.active_buffer_mut();
                let removed = buf.remove(top);

                // Save to scrollback only if on primary screen and scroll region is full screen
                if !self.using_alternate
                    && self.scroll_region_top == 0
                    && self.scroll_region_bottom == self.size.rows.saturating_sub(1)
                {
                    self.scrollback.push(removed);
                    if self.scrollback.len() > self.scrollback_limit {
                        self.scrollback.remove(0);
                    }
                }

                let blank_row: Vec<TerminalCell> = (0..cols as usize)
                    .map(|_| TerminalCell::default())
                    .collect();
                let buf = self.active_buffer_mut();
                buf.insert(bottom, blank_row);
            }
        }
    }

    /// Scroll the scroll region down by n lines.
    /// The bottom line(s) of the scroll region are removed and blank lines
    /// are inserted at the top of the scroll region.
    pub fn scroll_down(&mut self, n: u16) {
        let top = self.scroll_region_top as usize;
        let bottom = self.scroll_region_bottom as usize;
        let cols = self.size.cols;

        for _ in 0..n {
            if top <= bottom {
                let buf = self.active_buffer_mut();
                buf.remove(bottom);

                let blank_row: Vec<TerminalCell> = (0..cols as usize)
                    .map(|_| TerminalCell::default())
                    .collect();
                let buf = self.active_buffer_mut();
                buf.insert(top, blank_row);
            }
        }
    }

    /// Set the scroll region. Values are 0-based and inclusive.
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let max_bottom = self.size.rows.saturating_sub(1);
        self.scroll_region_top = top.min(max_bottom);
        self.scroll_region_bottom = bottom.min(max_bottom);
        if self.scroll_region_top > self.scroll_region_bottom {
            std::mem::swap(&mut self.scroll_region_top, &mut self.scroll_region_bottom);
        }
    }

    /// Erase the entire display.
    pub fn erase_display(&mut self) {
        let cols = self.size.cols;
        let rows = self.size.rows;
        *self.active_buffer_mut() = Self::create_buffer(cols, rows);
    }

    /// Erase display variants — 0: below cursor, 1: above cursor, 2: entire display.
    pub fn erase_display_variant(&mut self, mode: u16) {
        match mode {
            0 => {
                // Erase from cursor to end of display
                let row = self.cursor.row as usize;
                let col = self.cursor.col as usize;
                let cols = self.size.cols as usize;
                let rows = self.size.rows as usize;
                let buf = self.active_buffer_mut();
                // Clear rest of current line
                if row < rows {
                    for cell in buf[row].iter_mut().take(cols).skip(col) {
                        *cell = TerminalCell::default();
                    }
                }
                // Clear all lines below
                for row_buf in buf.iter_mut().take(rows).skip(row + 1) {
                    for cell in row_buf.iter_mut().take(cols) {
                        *cell = TerminalCell::default();
                    }
                }
            }
            1 => {
                // Erase from start to cursor
                let row = self.cursor.row as usize;
                let col = self.cursor.col as usize;
                let cols = self.size.cols as usize;
                let buf = self.active_buffer_mut();
                // Clear all lines above
                for row_buf in buf.iter_mut().take(row) {
                    for cell in row_buf.iter_mut().take(cols) {
                        *cell = TerminalCell::default();
                    }
                }
                // Clear current line up to cursor
                for cell in buf[row]
                    .iter_mut()
                    .take(col.min(cols.saturating_sub(1)) + 1)
                {
                    *cell = TerminalCell::default();
                }
            }
            2 | 3 => {
                self.erase_display();
            }
            _ => {}
        }
    }

    /// Erase the current line.
    pub fn erase_line(&mut self) {
        self.erase_line_variant(2);
    }

    /// Erase line variants — 0: right of cursor, 1: left of cursor, 2: entire line.
    pub fn erase_line_variant(&mut self, mode: u16) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        let cols = self.size.cols as usize;
        let buf = self.active_buffer_mut();

        if row >= buf.len() {
            return;
        }

        match mode {
            0 => {
                for cell in buf[row].iter_mut().take(cols).skip(col) {
                    *cell = TerminalCell::default();
                }
            }
            1 => {
                for cell in buf[row]
                    .iter_mut()
                    .take(col.min(cols.saturating_sub(1)) + 1)
                {
                    *cell = TerminalCell::default();
                }
            }
            2 => {
                for cell in buf[row].iter_mut().take(cols) {
                    *cell = TerminalCell::default();
                }
            }
            _ => {}
        }
    }

    /// Switch to the alternate screen buffer.
    pub fn enter_alternate_screen(&mut self) {
        if !self.using_alternate {
            self.saved_cursor = self.cursor;
            self.using_alternate = true;
            self.erase_display();
            self.cursor = CursorPosition::default();
        }
    }

    /// Switch back to the primary screen buffer.
    pub fn leave_alternate_screen(&mut self) {
        if self.using_alternate {
            self.using_alternate = false;
            self.cursor = self.saved_cursor;
        }
    }

    /// Whether the alternate screen is currently active.
    pub fn is_alternate_screen(&self) -> bool {
        self.using_alternate
    }

    /// Get a reference to the scrollback buffer.
    pub fn scrollback(&self) -> &[Vec<TerminalCell>] {
        &self.scrollback
    }

    /// Set the maximum scrollback size.
    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback_limit = limit;
    }

    /// Resize the grid to a new size.
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        self.size = TerminalSize::new(new_cols, new_rows);
        self.scroll_region_top = 0;
        self.scroll_region_bottom = new_rows.saturating_sub(1);

        // For simplicity, just create new buffers — real implementation would reflow.
        self.primary = Self::create_buffer(new_cols, new_rows);
        self.alternate = Self::create_buffer(new_cols, new_rows);

        // Clamp cursor
        self.cursor.row = self.cursor.row.min(new_rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(new_cols.saturating_sub(1));
    }

    /// Extract the text content of a given row as a String.
    pub fn row_text(&self, row: u16) -> String {
        let buf = self.active_buffer();
        if let Some(r) = buf.get(row as usize) {
            r.iter().map(|c| c.character).collect()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_new_has_correct_dimensions() {
        let grid = TerminalGrid::new(80, 24);
        assert_eq!(grid.size(), TerminalSize::new(80, 24));

        // All cells should be default (space, default colors, no attributes)
        for row in 0..24 {
            for col in 0..80 {
                let cell = grid.get_cell(row, col).unwrap();
                assert_eq!(cell.character, ' ');
                assert_eq!(cell.fg, Color::Default);
                assert_eq!(cell.bg, Color::Default);
                assert_eq!(cell.attrs, CellAttributes::default());
            }
        }
    }

    #[test]
    fn grid_set_cell_content() {
        let mut grid = TerminalGrid::new(80, 24);
        let cell = TerminalCell {
            character: 'A',
            fg: Color::Indexed(1),
            bg: Color::Default,
            attrs: CellAttributes::default(),
            hyperlink: None,
        };
        grid.set_cell(0, 0, cell.clone());

        let got = grid.get_cell(0, 0).unwrap();
        assert_eq!(got.character, 'A');
        assert_eq!(got.fg, Color::Indexed(1));
    }

    #[test]
    fn grid_cursor_move_right() {
        let mut grid = TerminalGrid::new(80, 24);
        assert_eq!(grid.cursor.col, 0);
        grid.cursor_right(5);
        assert_eq!(grid.cursor.col, 5);
    }

    #[test]
    fn grid_cursor_wraps_at_eol() {
        let mut grid = TerminalGrid::new(5, 3);
        // Write 6 characters into a 5-col grid — should wrap to next row
        for ch in "ABCDEF".chars() {
            grid.write_char(ch);
        }
        // First row: "ABCDE"
        assert_eq!(grid.row_text(0).trim_end(), "ABCDE");
        // Second row starts with 'F'
        let cell = grid.get_cell(1, 0).unwrap();
        assert_eq!(cell.character, 'F');
    }

    #[test]
    fn grid_scroll_up() {
        let mut grid = TerminalGrid::new(5, 3);
        // Fill rows with identifiable chars
        grid.cursor_set_position(0, 0);
        grid.write_char('A');
        grid.cursor_set_position(1, 0);
        grid.write_char('B');
        grid.cursor_set_position(2, 0);
        grid.write_char('C');

        grid.scroll_up(1);

        // Row 0 should now be what was row 1 (B)
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'B');
        // Row 1 should now be what was row 2 (C)
        assert_eq!(grid.get_cell(1, 0).unwrap().character, 'C');
        // Row 2 should be blank
        assert_eq!(grid.get_cell(2, 0).unwrap().character, ' ');
    }

    #[test]
    fn grid_scroll_down() {
        let mut grid = TerminalGrid::new(5, 3);
        grid.cursor_set_position(0, 0);
        grid.write_char('A');
        grid.cursor_set_position(1, 0);
        grid.write_char('B');
        grid.cursor_set_position(2, 0);
        grid.write_char('C');

        grid.scroll_down(1);

        // Row 0 should be blank (inserted)
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');
        // Row 1 should be what was row 0 (A)
        assert_eq!(grid.get_cell(1, 0).unwrap().character, 'A');
        // Row 2 should be what was row 1 (B)
        assert_eq!(grid.get_cell(2, 0).unwrap().character, 'B');
        // Old row 2 (C) was removed
    }

    #[test]
    fn grid_alternate_screen_switch() {
        let mut grid = TerminalGrid::new(80, 24);
        // Write something on primary
        grid.cursor_set_position(0, 0);
        grid.write_char('X');
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'X');

        // Switch to alternate
        grid.enter_alternate_screen();
        assert!(grid.is_alternate_screen());
        // Alternate should be blank
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');

        // Write on alternate
        grid.cursor_set_position(0, 0);
        grid.write_char('Y');
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'Y');

        // Switch back to primary
        grid.leave_alternate_screen();
        assert!(!grid.is_alternate_screen());
        // Primary content should be preserved
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'X');
    }

    #[test]
    fn grid_erase_display() {
        let mut grid = TerminalGrid::new(80, 24);
        grid.write_char('Z');
        grid.erase_display();
        assert_eq!(grid.get_cell(0, 0).unwrap().character, ' ');
    }

    #[test]
    fn grid_erase_line() {
        let mut grid = TerminalGrid::new(10, 3);
        grid.cursor_set_position(1, 0);
        for ch in "Hello".chars() {
            grid.write_char(ch);
        }
        grid.cursor_set_position(1, 0);
        grid.erase_line();

        for col in 0..10 {
            assert_eq!(grid.get_cell(1, col).unwrap().character, ' ');
        }
    }

    #[test]
    fn grid_scrollback_preserved() {
        let mut grid = TerminalGrid::new(5, 3);
        grid.set_scrollback_limit(100);

        // Write 'A' on row 0
        grid.cursor_set_position(0, 0);
        grid.write_char('A');

        // Scroll up — row 0 ('A') goes to scrollback
        grid.scroll_up(1);

        assert_eq!(grid.scrollback().len(), 1);
        assert_eq!(grid.scrollback()[0][0].character, 'A');
    }

    #[test]
    fn grid_cursor_clamps() {
        let mut grid = TerminalGrid::new(10, 5);
        grid.cursor_up(100);
        assert_eq!(grid.cursor.row, 0);
        grid.cursor_left(100);
        assert_eq!(grid.cursor.col, 0);
        grid.cursor_down(100);
        assert_eq!(grid.cursor.row, 4);
        grid.cursor_right(100);
        assert_eq!(grid.cursor.col, 9);
    }

    #[test]
    fn grid_set_scroll_region() {
        let mut grid = TerminalGrid::new(80, 24);
        grid.set_scroll_region(4, 19);
        // Scroll should only affect the region
        grid.cursor_set_position(4, 0);
        grid.write_char('T');
        grid.cursor_set_position(19, 0);
        grid.write_char('B');

        // Verify cells
        assert_eq!(grid.get_cell(4, 0).unwrap().character, 'T');
        assert_eq!(grid.get_cell(19, 0).unwrap().character, 'B');
    }

    #[test]
    fn grid_row_text() {
        let mut grid = TerminalGrid::new(10, 3);
        grid.cursor_set_position(0, 0);
        for ch in "Hello".chars() {
            grid.write_char(ch);
        }
        let text = grid.row_text(0);
        assert!(text.starts_with("Hello"));
    }

    #[test]
    fn grid_get_cell_out_of_bounds() {
        let grid = TerminalGrid::new(10, 5);
        assert!(grid.get_cell(10, 0).is_none());
        assert!(grid.get_cell(0, 20).is_none());
    }

    #[test]
    fn grid_erase_display_variant_0() {
        let mut grid = TerminalGrid::new(10, 3);
        // Fill row 0 and row 2
        grid.cursor_set_position(0, 0);
        grid.write_char('A');
        grid.cursor_set_position(2, 0);
        grid.write_char('C');
        // Set cursor at row 1, col 0 and erase below
        grid.cursor_set_position(1, 0);
        grid.erase_display_variant(0);
        // Row 0 should be preserved
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        // Row 2 should be erased
        assert_eq!(grid.get_cell(2, 0).unwrap().character, ' ');
    }

    #[test]
    fn grid_erase_line_variant_0() {
        let mut grid = TerminalGrid::new(10, 3);
        grid.cursor_set_position(0, 0);
        for ch in "ABCDE".chars() {
            grid.write_char(ch);
        }
        // Cursor at col 2, erase right
        grid.cursor_set_position(0, 2);
        grid.erase_line_variant(0);
        assert_eq!(grid.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(grid.get_cell(0, 1).unwrap().character, 'B');
        assert_eq!(grid.get_cell(0, 2).unwrap().character, ' ');
        assert_eq!(grid.get_cell(0, 3).unwrap().character, ' ');
    }
}
