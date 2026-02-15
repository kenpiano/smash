/// Tracks which portion of a buffer is visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewport {
    /// First visible line (0-based).
    top_line: usize,
    /// First visible column for horizontal scroll (0-based).
    left_col: usize,
    /// Number of visible lines.
    visible_lines: usize,
    /// Number of visible columns.
    visible_cols: usize,
}

impl Viewport {
    pub fn new(visible_lines: usize, visible_cols: usize) -> Self {
        Self {
            top_line: 0,
            left_col: 0,
            visible_lines,
            visible_cols,
        }
    }

    pub fn top_line(&self) -> usize {
        self.top_line
    }
    pub fn left_col(&self) -> usize {
        self.left_col
    }
    pub fn visible_lines(&self) -> usize {
        self.visible_lines
    }
    pub fn visible_cols(&self) -> usize {
        self.visible_cols
    }
    pub fn bottom_line(&self) -> usize {
        self.top_line + self.visible_lines
    }

    /// Ensure cursor line is visible, scrolling if needed.
    pub fn scroll_to_cursor(&mut self, cursor_line: usize, cursor_col: usize) {
        // Vertical scroll
        if cursor_line < self.top_line {
            self.top_line = cursor_line;
        } else if cursor_line >= self.top_line + self.visible_lines {
            self.top_line = cursor_line.saturating_sub(self.visible_lines - 1);
        }
        // Horizontal scroll
        if cursor_col < self.left_col {
            self.left_col = cursor_col;
        } else if cursor_col >= self.left_col + self.visible_cols {
            self.left_col = cursor_col.saturating_sub(self.visible_cols - 1);
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.top_line = self.top_line.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, total_lines: usize) {
        let max = total_lines.saturating_sub(self.visible_lines);
        self.top_line = (self.top_line + lines).min(max);
    }

    pub fn resize(&mut self, lines: usize, cols: usize) {
        self.visible_lines = lines;
        self.visible_cols = cols;
    }

    pub fn set_top_line(&mut self, line: usize) {
        self.top_line = line;
    }
    pub fn set_left_col(&mut self, col: usize) {
        self.left_col = col;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_new_defaults() {
        let vp = Viewport::new(24, 80);
        assert_eq!(vp.top_line(), 0);
        assert_eq!(vp.left_col(), 0);
        assert_eq!(vp.visible_lines(), 24);
        assert_eq!(vp.visible_cols(), 80);
    }

    #[test]
    fn viewport_bottom_line() {
        let vp = Viewport::new(24, 80);
        assert_eq!(vp.bottom_line(), 24);
    }

    #[test]
    fn scroll_to_cursor_no_scroll_when_in_view() {
        let mut vp = Viewport::new(10, 80);
        vp.scroll_to_cursor(5, 10);
        assert_eq!(vp.top_line(), 0);
        assert_eq!(vp.left_col(), 0);
    }

    #[test]
    fn scroll_to_cursor_scrolls_down() {
        let mut vp = Viewport::new(10, 80);
        vp.scroll_to_cursor(15, 0);
        assert_eq!(vp.top_line(), 6); // 15 - (10-1) = 6
    }

    #[test]
    fn scroll_to_cursor_scrolls_up() {
        let mut vp = Viewport::new(10, 80);
        vp.set_top_line(20);
        vp.scroll_to_cursor(5, 0);
        assert_eq!(vp.top_line(), 5);
    }

    #[test]
    fn scroll_to_cursor_horizontal_right() {
        let mut vp = Viewport::new(10, 20);
        vp.scroll_to_cursor(0, 25);
        assert_eq!(vp.left_col(), 6); // 25 - (20-1) = 6
    }

    #[test]
    fn scroll_to_cursor_horizontal_left() {
        let mut vp = Viewport::new(10, 20);
        vp.set_left_col(10);
        vp.scroll_to_cursor(0, 5);
        assert_eq!(vp.left_col(), 5);
    }

    #[test]
    fn scroll_up_basic() {
        let mut vp = Viewport::new(10, 80);
        vp.set_top_line(20);
        vp.scroll_up(5);
        assert_eq!(vp.top_line(), 15);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut vp = Viewport::new(10, 80);
        vp.set_top_line(3);
        vp.scroll_up(10);
        assert_eq!(vp.top_line(), 0);
    }

    #[test]
    fn scroll_down_basic() {
        let mut vp = Viewport::new(10, 80);
        vp.scroll_down(5, 100);
        assert_eq!(vp.top_line(), 5);
    }

    #[test]
    fn scroll_down_clamps_to_max() {
        let mut vp = Viewport::new(10, 80);
        vp.scroll_down(100, 30);
        // max = 30 - 10 = 20
        assert_eq!(vp.top_line(), 20);
    }

    #[test]
    fn scroll_down_total_less_than_visible() {
        let mut vp = Viewport::new(10, 80);
        vp.scroll_down(5, 5);
        // max = 5.saturating_sub(10) = 0
        assert_eq!(vp.top_line(), 0);
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut vp = Viewport::new(10, 80);
        vp.resize(20, 100);
        assert_eq!(vp.visible_lines(), 20);
        assert_eq!(vp.visible_cols(), 100);
    }

    #[test]
    fn set_top_line_and_left_col() {
        let mut vp = Viewport::new(10, 80);
        vp.set_top_line(42);
        vp.set_left_col(7);
        assert_eq!(vp.top_line(), 42);
        assert_eq!(vp.left_col(), 7);
    }
}
