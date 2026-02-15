use crate::style::Style;

/// A single terminal cell with a character and style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Cell {
    pub fn new(ch: char, style: Style) -> Self {
        Self { ch, style }
    }

    pub fn blank() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }

    pub fn with_char(ch: char) -> Self {
        Self {
            ch,
            style: Style::default(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Attributes, Color};

    #[test]
    fn blank_cell_is_space_with_default_style() {
        let cell = Cell::blank();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.style, Style::default());
    }

    #[test]
    fn with_char_uses_default_style() {
        let cell = Cell::with_char('A');
        assert_eq!(cell.ch, 'A');
        assert_eq!(cell.style, Style::default());
    }

    #[test]
    fn default_equals_blank() {
        assert_eq!(Cell::default(), Cell::blank());
    }

    #[test]
    fn new_cell_with_style() {
        let style = Style::default().fg(Color::Red).bold();
        let cell = Cell::new('X', style);
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.style.fg, Color::Red);
        assert!(cell.style.attrs.bold());
    }

    #[test]
    fn cell_clone_equality() {
        let cell = Cell::new(
            'Z',
            Style::new(Color::Green, Color::Black, Attributes::ITALIC),
        );
        let cloned = cell.clone();
        assert_eq!(cell, cloned);
    }

    #[test]
    fn cell_inequality() {
        let a = Cell::with_char('A');
        let b = Cell::with_char('B');
        assert_ne!(a, b);
    }

    #[test]
    fn cell_debug_format() {
        let cell = Cell::blank();
        let debug = format!("{:?}", cell);
        assert!(debug.contains("Cell"));
    }
}
