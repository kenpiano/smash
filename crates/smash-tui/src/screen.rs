use crate::cell::Cell;
use crate::style::Style;

/// A 2D grid of cells representing the terminal screen.
#[derive(Debug, Clone)]
pub struct Screen {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Screen {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            cells: vec![Cell::blank(); size],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn get(&self, col: u16, row: u16) -> Option<&Cell> {
        if col < self.width && row < self.height {
            Some(&self.cells[self.index(col, row)])
        } else {
            None
        }
    }

    pub fn set(&mut self, col: u16, row: u16, cell: Cell) {
        if col < self.width && row < self.height {
            let idx = self.index(col, row);
            self.cells[idx] = cell;
        }
    }

    pub fn put_char(&mut self, col: u16, row: u16, ch: char, style: Style) {
        self.set(col, row, Cell::new(ch, style));
    }

    pub fn put_str(&mut self, col: u16, row: u16, s: &str, style: Style) {
        let mut c = col;
        for ch in s.chars() {
            if c >= self.width {
                break;
            }
            self.set(c, row, Cell::new(ch, style));
            c += 1;
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::blank();
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let size = (width as usize) * (height as usize);
        self.cells = vec![Cell::blank(); size];
    }

    /// Compute diff: returns `(col, row, Cell)` for cells that
    /// differ between `self` and `other`.
    pub fn diff(&self, other: &Screen) -> Vec<(u16, u16, Cell)> {
        let mut changes = Vec::new();
        if self.width != other.width || self.height != other.height {
            // Full repaint needed; return all cells of `other`
            for row in 0..other.height {
                for col in 0..other.width {
                    if let Some(cell) = other.get(col, row) {
                        changes.push((col, row, cell.clone()));
                    }
                }
            }
            return changes;
        }
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.index(col, row);
                if self.cells[idx] != other.cells[idx] {
                    changes.push((col, row, other.cells[idx].clone()));
                }
            }
        }
        changes
    }

    fn index(&self, col: u16, row: u16) -> usize {
        (row as usize) * (self.width as usize) + (col as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Color;

    #[test]
    fn screen_new_correct_dimensions() {
        let s = Screen::new(80, 24);
        assert_eq!(s.width(), 80);
        assert_eq!(s.height(), 24);
    }

    #[test]
    fn screen_new_cells_are_blank() {
        let s = Screen::new(10, 5);
        for row in 0..5 {
            for col in 0..10 {
                let cell = s.get(col, row).unwrap();
                assert_eq!(*cell, Cell::blank());
            }
        }
    }

    #[test]
    fn screen_get_out_of_bounds_returns_none() {
        let s = Screen::new(10, 5);
        assert!(s.get(10, 0).is_none());
        assert!(s.get(0, 5).is_none());
        assert!(s.get(100, 100).is_none());
    }

    #[test]
    fn screen_set_and_get() {
        let mut s = Screen::new(10, 5);
        let cell = Cell::with_char('A');
        s.set(3, 2, cell.clone());
        assert_eq!(s.get(3, 2).unwrap(), &cell);
    }

    #[test]
    fn screen_set_out_of_bounds_is_noop() {
        let mut s = Screen::new(10, 5);
        s.set(100, 100, Cell::with_char('X'));
        // Should not panic or corrupt
        assert_eq!(s.get(0, 0).unwrap(), &Cell::blank());
    }

    #[test]
    fn screen_put_char() {
        let mut s = Screen::new(10, 5);
        let style = Style::default().fg(Color::Red);
        s.put_char(5, 3, 'Z', style);
        let cell = s.get(5, 3).unwrap();
        assert_eq!(cell.ch, 'Z');
        assert_eq!(cell.style.fg, Color::Red);
    }

    #[test]
    fn screen_put_str() {
        let mut s = Screen::new(20, 5);
        let style = Style::default();
        s.put_str(2, 1, "Hello", style);
        assert_eq!(s.get(2, 1).unwrap().ch, 'H');
        assert_eq!(s.get(3, 1).unwrap().ch, 'e');
        assert_eq!(s.get(4, 1).unwrap().ch, 'l');
        assert_eq!(s.get(5, 1).unwrap().ch, 'l');
        assert_eq!(s.get(6, 1).unwrap().ch, 'o');
    }

    #[test]
    fn screen_put_str_truncates_at_width() {
        let mut s = Screen::new(5, 1);
        s.put_str(3, 0, "Hello", Style::default());
        assert_eq!(s.get(3, 0).unwrap().ch, 'H');
        assert_eq!(s.get(4, 0).unwrap().ch, 'e');
        // "llo" did not fit
    }

    #[test]
    fn screen_clear_resets_all_cells() {
        let mut s = Screen::new(10, 5);
        s.set(3, 2, Cell::with_char('X'));
        s.clear();
        assert_eq!(s.get(3, 2).unwrap(), &Cell::blank());
    }

    #[test]
    fn screen_resize_resets_cells() {
        let mut s = Screen::new(10, 5);
        s.set(0, 0, Cell::with_char('A'));
        s.resize(20, 10);
        assert_eq!(s.width(), 20);
        assert_eq!(s.height(), 10);
        assert_eq!(s.get(0, 0).unwrap(), &Cell::blank());
    }

    #[test]
    fn screen_diff_identical_screens_empty() {
        let a = Screen::new(10, 5);
        let b = Screen::new(10, 5);
        assert!(a.diff(&b).is_empty());
    }

    #[test]
    fn screen_diff_returns_changed_cells() {
        let a = Screen::new(10, 5);
        let mut b = Screen::new(10, 5);
        b.set(3, 2, Cell::with_char('X'));
        b.set(7, 4, Cell::with_char('Y'));
        let diff = a.diff(&b);
        assert_eq!(diff.len(), 2);
        assert!(diff.contains(&(3, 2, Cell::with_char('X'))));
        assert!(diff.contains(&(7, 4, Cell::with_char('Y'))));
    }

    #[test]
    fn screen_diff_different_sizes_returns_all() {
        let a = Screen::new(10, 5);
        let b = Screen::new(20, 10);
        let diff = a.diff(&b);
        assert_eq!(diff.len(), 200);
    }
}
