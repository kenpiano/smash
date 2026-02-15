use crate::cell::Cell;
use crate::error::TuiError;

/// Trait for terminal output (real or mock).
pub trait TerminalBackend {
    fn size(&self) -> Result<(u16, u16), TuiError>;
    fn move_cursor(&mut self, col: u16, row: u16) -> Result<(), TuiError>;
    fn show_cursor(&mut self) -> Result<(), TuiError>;
    fn hide_cursor(&mut self) -> Result<(), TuiError>;
    fn clear(&mut self) -> Result<(), TuiError>;
    fn write_cell(&mut self, col: u16, row: u16, cell: &Cell) -> Result<(), TuiError>;
    fn flush(&mut self) -> Result<(), TuiError>;
    fn enter_alternate_screen(&mut self) -> Result<(), TuiError>;
    fn leave_alternate_screen(&mut self) -> Result<(), TuiError>;
    fn enable_raw_mode(&mut self) -> Result<(), TuiError>;
    fn disable_raw_mode(&mut self) -> Result<(), TuiError>;
}

/// Mock backend for testing — records all operations.
#[derive(Debug)]
pub struct MockBackend {
    width: u16,
    height: u16,
    cursor: (u16, u16),
    cursor_visible: bool,
    cells: Vec<Vec<Cell>>,
    raw_mode: bool,
    alternate_screen: bool,
    pub flush_count: usize,
}

impl MockBackend {
    pub fn new(width: u16, height: u16) -> Self {
        let cells = (0..height)
            .map(|_| vec![Cell::blank(); width as usize])
            .collect();
        Self {
            width,
            height,
            cursor: (0, 0),
            cursor_visible: true,
            cells,
            raw_mode: false,
            alternate_screen: false,
            flush_count: 0,
        }
    }

    pub fn cell_at(&self, col: u16, row: u16) -> Option<&Cell> {
        self.cells
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        self.cursor
    }
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }
    pub fn is_raw_mode(&self) -> bool {
        self.raw_mode
    }
    pub fn is_alternate_screen(&self) -> bool {
        self.alternate_screen
    }

    /// Read a string from the mock screen at given row.
    pub fn read_row(&self, row: u16) -> String {
        self.cells
            .get(row as usize)
            .map(|r| r.iter().map(|c| c.ch).collect::<String>())
            .unwrap_or_default()
            .trim_end()
            .to_string()
    }
}

impl TerminalBackend for MockBackend {
    fn size(&self) -> Result<(u16, u16), TuiError> {
        Ok((self.width, self.height))
    }

    fn move_cursor(&mut self, col: u16, row: u16) -> Result<(), TuiError> {
        self.cursor = (col, row);
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), TuiError> {
        self.cursor_visible = true;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), TuiError> {
        self.cursor_visible = false;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), TuiError> {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::blank();
            }
        }
        Ok(())
    }

    fn write_cell(&mut self, col: u16, row: u16, cell: &Cell) -> Result<(), TuiError> {
        if let Some(r) = self.cells.get_mut(row as usize) {
            if let Some(c) = r.get_mut(col as usize) {
                *c = cell.clone();
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), TuiError> {
        self.flush_count += 1;
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.alternate_screen = true;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.alternate_screen = false;
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> Result<(), TuiError> {
        self.raw_mode = true;
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<(), TuiError> {
        self.raw_mode = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Color, Style};

    #[test]
    fn mock_backend_size() {
        let b = MockBackend::new(80, 24);
        assert_eq!(b.size().unwrap(), (80, 24));
    }

    #[test]
    fn mock_backend_write_cell_and_read() {
        let mut b = MockBackend::new(80, 24);
        let cell = Cell::new('X', Style::default().fg(Color::Red));
        b.write_cell(5, 3, &cell).unwrap();
        let read = b.cell_at(5, 3).unwrap();
        assert_eq!(read.ch, 'X');
        assert_eq!(read.style.fg, Color::Red);
    }

    #[test]
    fn mock_backend_read_row() {
        let mut b = MockBackend::new(10, 5);
        let style = Style::default();
        b.write_cell(0, 0, &Cell::new('H', style)).unwrap();
        b.write_cell(1, 0, &Cell::new('i', style)).unwrap();
        assert_eq!(b.read_row(0), "Hi");
    }

    #[test]
    fn mock_backend_cursor_control() {
        let mut b = MockBackend::new(80, 24);
        assert!(b.is_cursor_visible());
        b.hide_cursor().unwrap();
        assert!(!b.is_cursor_visible());
        b.show_cursor().unwrap();
        assert!(b.is_cursor_visible());
    }

    #[test]
    fn mock_backend_move_cursor() {
        let mut b = MockBackend::new(80, 24);
        b.move_cursor(10, 5).unwrap();
        assert_eq!(b.cursor_position(), (10, 5));
    }

    #[test]
    fn mock_backend_raw_mode_toggle() {
        let mut b = MockBackend::new(80, 24);
        assert!(!b.is_raw_mode());
        b.enable_raw_mode().unwrap();
        assert!(b.is_raw_mode());
        b.disable_raw_mode().unwrap();
        assert!(!b.is_raw_mode());
    }

    #[test]
    fn mock_backend_alternate_screen_toggle() {
        let mut b = MockBackend::new(80, 24);
        assert!(!b.is_alternate_screen());
        b.enter_alternate_screen().unwrap();
        assert!(b.is_alternate_screen());
        b.leave_alternate_screen().unwrap();
        assert!(!b.is_alternate_screen());
    }

    #[test]
    fn mock_backend_clear_resets_cells() {
        let mut b = MockBackend::new(10, 5);
        b.write_cell(0, 0, &Cell::with_char('X')).unwrap();
        b.clear().unwrap();
        assert_eq!(b.cell_at(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn mock_backend_flush_count() {
        let mut b = MockBackend::new(80, 24);
        assert_eq!(b.flush_count, 0);
        b.flush().unwrap();
        b.flush().unwrap();
        assert_eq!(b.flush_count, 2);
    }

    #[test]
    fn mock_backend_cell_at_out_of_bounds() {
        let b = MockBackend::new(10, 5);
        assert!(b.cell_at(10, 0).is_none());
        assert!(b.cell_at(0, 5).is_none());
    }

    #[test]
    fn mock_backend_write_cell_out_of_bounds_no_panic() {
        let mut b = MockBackend::new(10, 5);
        // Should not panic
        b.write_cell(100, 100, &Cell::blank()).unwrap();
    }
}
