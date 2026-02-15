/// Terminal colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Color {
    #[default]
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

/// Text attributes stored as a bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Attributes(u8);

impl Attributes {
    pub const NONE: Self = Self(0);
    pub const BOLD: Self = Self(1);
    pub const DIM: Self = Self(2);
    pub const ITALIC: Self = Self(4);
    pub const UNDERLINE: Self = Self(8);
    pub const REVERSE: Self = Self(16);
    pub const STRIKETHROUGH: Self = Self(32);

    pub fn bold(self) -> bool {
        self.0 & 1 != 0
    }
    pub fn dim(self) -> bool {
        self.0 & 2 != 0
    }
    pub fn italic(self) -> bool {
        self.0 & 4 != 0
    }
    pub fn underline(self) -> bool {
        self.0 & 8 != 0
    }
    pub fn reverse(self) -> bool {
        self.0 & 16 != 0
    }
    pub fn strikethrough(self) -> bool {
        self.0 & 32 != 0
    }
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for Attributes {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Complete cell style: foreground, background, and attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
}

impl Style {
    pub fn new(fg: Color, bg: Color, attrs: Attributes) -> Self {
        Self { fg, bg, attrs }
    }
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }
    pub fn bold(mut self) -> Self {
        self.attrs = self.attrs | Attributes::BOLD;
        self
    }
    pub fn italic(mut self) -> Self {
        self.attrs = self.attrs | Attributes::ITALIC;
        self
    }
    pub fn underline(mut self) -> Self {
        self.attrs = self.attrs | Attributes::UNDERLINE;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_default_is_reset() {
        assert_eq!(Color::default(), Color::Reset);
    }

    #[test]
    fn attributes_none_has_no_flags() {
        let a = Attributes::NONE;
        assert!(!a.bold());
        assert!(!a.dim());
        assert!(!a.italic());
        assert!(!a.underline());
        assert!(!a.reverse());
        assert!(!a.strikethrough());
    }

    #[test]
    fn attributes_bitor_combines_flags() {
        let a = Attributes::BOLD | Attributes::ITALIC;
        assert!(a.bold());
        assert!(a.italic());
        assert!(!a.dim());
    }

    #[test]
    fn attributes_contains_checks_subset() {
        let a = Attributes::BOLD | Attributes::ITALIC;
        assert!(a.contains(Attributes::BOLD));
        assert!(a.contains(Attributes::ITALIC));
        assert!(a.contains(Attributes::BOLD | Attributes::ITALIC));
        assert!(!a.contains(Attributes::DIM));
    }

    #[test]
    fn attributes_default_is_none() {
        assert_eq!(Attributes::default(), Attributes::NONE);
    }

    #[test]
    fn style_default_has_reset_colors_and_no_attrs() {
        let s = Style::default();
        assert_eq!(s.fg, Color::Reset);
        assert_eq!(s.bg, Color::Reset);
        assert_eq!(s.attrs, Attributes::NONE);
    }

    #[test]
    fn style_builder_fg() {
        let s = Style::default().fg(Color::Red);
        assert_eq!(s.fg, Color::Red);
        assert_eq!(s.bg, Color::Reset);
    }

    #[test]
    fn style_builder_bg() {
        let s = Style::default().bg(Color::Blue);
        assert_eq!(s.bg, Color::Blue);
    }

    #[test]
    fn style_builder_bold() {
        let s = Style::default().bold();
        assert!(s.attrs.bold());
    }

    #[test]
    fn style_builder_italic() {
        let s = Style::default().italic();
        assert!(s.attrs.italic());
    }

    #[test]
    fn style_builder_underline() {
        let s = Style::default().underline();
        assert!(s.attrs.underline());
    }

    #[test]
    fn style_builder_chained() {
        let s = Style::default()
            .fg(Color::Green)
            .bg(Color::Black)
            .bold()
            .italic();
        assert_eq!(s.fg, Color::Green);
        assert_eq!(s.bg, Color::Black);
        assert!(s.attrs.bold());
        assert!(s.attrs.italic());
    }

    #[test]
    fn style_new_constructor() {
        let s = Style::new(
            Color::White,
            Color::Black,
            Attributes::BOLD | Attributes::UNDERLINE,
        );
        assert_eq!(s.fg, Color::White);
        assert_eq!(s.bg, Color::Black);
        assert!(s.attrs.bold());
        assert!(s.attrs.underline());
    }

    #[test]
    fn color_rgb_equality() {
        assert_eq!(Color::Rgb(10, 20, 30), Color::Rgb(10, 20, 30));
        assert_ne!(Color::Rgb(10, 20, 30), Color::Rgb(10, 20, 31));
    }

    #[test]
    fn color_indexed_equality() {
        assert_eq!(Color::Indexed(42), Color::Indexed(42));
        assert_ne!(Color::Indexed(42), Color::Indexed(43));
    }

    #[test]
    fn attributes_individual_flags() {
        assert!(Attributes::BOLD.bold());
        assert!(Attributes::DIM.dim());
        assert!(Attributes::ITALIC.italic());
        assert!(Attributes::UNDERLINE.underline());
        assert!(Attributes::REVERSE.reverse());
        assert!(Attributes::STRIKETHROUGH.strikethrough());
    }
}
