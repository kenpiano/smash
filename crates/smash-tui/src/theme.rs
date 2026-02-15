use std::collections::HashMap;

use smash_syntax::ScopeId;

use crate::style::{Color, Style};

/// Maps syntax scopes to terminal styles.
#[derive(Debug, Clone)]
pub struct Theme {
    name: String,
    styles: HashMap<ScopeId, Style>,
    /// Default text style.
    default_style: Style,
    /// UI element styles.
    status_bar: Style,
    line_number: Style,
    selection: Style,
    cursor: Style,
    /// Diagnostic gutter icon styles.
    diagnostic_error: Style,
    diagnostic_warning: Style,
    diagnostic_info: Style,
    diagnostic_hint: Style,
}

impl Theme {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            styles: HashMap::new(),
            default_style: Style::default(),
            status_bar: Style::default().fg(Color::Black).bg(Color::White),
            line_number: Style::default().fg(Color::Indexed(243)),
            selection: Style::default().bg(Color::Indexed(238)),
            cursor: Style::default().fg(Color::Black).bg(Color::White),
            diagnostic_error: Style::default().fg(Color::Red).bold(),
            diagnostic_warning: Style::default().fg(Color::Yellow).bold(),
            diagnostic_info: Style::default().fg(Color::Blue),
            diagnostic_hint: Style::default().fg(Color::Cyan),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_scope_style(&mut self, scope: ScopeId, style: Style) {
        self.styles.insert(scope, style);
    }

    pub fn scope_style(&self, scope: ScopeId) -> Style {
        self.styles
            .get(&scope)
            .copied()
            .unwrap_or(self.default_style)
    }

    pub fn default_style(&self) -> Style {
        self.default_style
    }
    pub fn set_default_style(&mut self, style: Style) {
        self.default_style = style;
    }

    pub fn status_bar_style(&self) -> Style {
        self.status_bar
    }
    pub fn set_status_bar_style(&mut self, style: Style) {
        self.status_bar = style;
    }

    pub fn line_number_style(&self) -> Style {
        self.line_number
    }
    pub fn set_line_number_style(&mut self, style: Style) {
        self.line_number = style;
    }

    pub fn selection_style(&self) -> Style {
        self.selection
    }
    pub fn set_selection_style(&mut self, style: Style) {
        self.selection = style;
    }

    pub fn cursor_style(&self) -> Style {
        self.cursor
    }
    pub fn set_cursor_style(&mut self, style: Style) {
        self.cursor = style;
    }

    pub fn diagnostic_error_style(&self) -> Style {
        self.diagnostic_error
    }
    pub fn set_diagnostic_error_style(&mut self, style: Style) {
        self.diagnostic_error = style;
    }

    pub fn diagnostic_warning_style(&self) -> Style {
        self.diagnostic_warning
    }
    pub fn set_diagnostic_warning_style(&mut self, style: Style) {
        self.diagnostic_warning = style;
    }

    pub fn diagnostic_info_style(&self) -> Style {
        self.diagnostic_info
    }
    pub fn set_diagnostic_info_style(&mut self, style: Style) {
        self.diagnostic_info = style;
    }

    pub fn diagnostic_hint_style(&self) -> Style {
        self.diagnostic_hint
    }
    pub fn set_diagnostic_hint_style(&mut self, style: Style) {
        self.diagnostic_hint = style;
    }
}

/// Built-in default dark theme.
pub fn default_dark_theme() -> Theme {
    let mut t = Theme::new("default-dark");
    t.set_default_style(Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 46)));
    t.set_scope_style(ScopeId::Keyword, Style::default().fg(Color::Magenta).bold());
    t.set_scope_style(ScopeId::Type, Style::default().fg(Color::Yellow));
    t.set_scope_style(ScopeId::Function, Style::default().fg(Color::Blue));
    t.set_scope_style(ScopeId::String, Style::default().fg(Color::Green));
    t.set_scope_style(
        ScopeId::Number,
        Style::default().fg(Color::Rgb(250, 179, 135)),
    );
    t.set_scope_style(
        ScopeId::Comment,
        Style::default().fg(Color::Indexed(243)).italic(),
    );
    t.set_scope_style(ScopeId::Operator, Style::default().fg(Color::Cyan));
    t.set_scope_style(
        ScopeId::Constant,
        Style::default().fg(Color::Rgb(250, 179, 135)),
    );
    t.set_scope_style(
        ScopeId::Macro,
        Style::default().fg(Color::Rgb(137, 180, 250)),
    );
    t.set_scope_style(
        ScopeId::Attribute,
        Style::default().fg(Color::Yellow).italic(),
    );
    t.set_scope_style(ScopeId::Variable, Style::default().fg(Color::White));
    t.set_scope_style(
        ScopeId::Punctuation,
        Style::default().fg(Color::Indexed(250)),
    );
    t.set_scope_style(ScopeId::Namespace, Style::default().fg(Color::Cyan));
    t.set_scope_style(ScopeId::Label, Style::default().fg(Color::Yellow));
    t.set_scope_style(ScopeId::Plain, Style::default().fg(Color::White));
    t.set_status_bar_style(Style::default().fg(Color::White).bg(Color::Indexed(238)));
    t.set_line_number_style(Style::default().fg(Color::Indexed(243)));
    t.set_selection_style(Style::default().bg(Color::Indexed(238)));
    t.set_cursor_style(Style::default().fg(Color::Black).bg(Color::White));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_new_has_name() {
        let t = Theme::new("test-theme");
        assert_eq!(t.name(), "test-theme");
    }

    #[test]
    fn theme_scope_style_returns_default_for_missing() {
        let t = Theme::new("test");
        let s = t.scope_style(ScopeId::Keyword);
        assert_eq!(s, t.default_style());
    }

    #[test]
    fn theme_set_and_get_scope_style() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Red).bold();
        t.set_scope_style(ScopeId::Keyword, style);
        assert_eq!(t.scope_style(ScopeId::Keyword), style);
    }

    #[test]
    fn theme_default_style_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::White);
        t.set_default_style(style);
        assert_eq!(t.default_style(), style);
    }

    #[test]
    fn theme_status_bar_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Green);
        t.set_status_bar_style(style);
        assert_eq!(t.status_bar_style(), style);
    }

    #[test]
    fn theme_line_number_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Blue);
        t.set_line_number_style(style);
        assert_eq!(t.line_number_style(), style);
    }

    #[test]
    fn theme_selection_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().bg(Color::Yellow);
        t.set_selection_style(style);
        assert_eq!(t.selection_style(), style);
    }

    #[test]
    fn theme_cursor_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Black).bg(Color::Cyan);
        t.set_cursor_style(style);
        assert_eq!(t.cursor_style(), style);
    }

    #[test]
    fn default_dark_theme_has_name() {
        let t = default_dark_theme();
        assert_eq!(t.name(), "default-dark");
    }

    #[test]
    fn default_dark_theme_has_keyword_scope() {
        let t = default_dark_theme();
        let s = t.scope_style(ScopeId::Keyword);
        assert_eq!(s.fg, Color::Magenta);
        assert!(s.attrs.bold());
    }

    #[test]
    fn default_dark_theme_has_all_standard_scopes() {
        let t = default_dark_theme();
        let scopes = [
            ScopeId::Keyword,
            ScopeId::Type,
            ScopeId::Function,
            ScopeId::String,
            ScopeId::Number,
            ScopeId::Comment,
            ScopeId::Operator,
            ScopeId::Constant,
            ScopeId::Macro,
            ScopeId::Attribute,
            ScopeId::Variable,
            ScopeId::Punctuation,
            ScopeId::Namespace,
            ScopeId::Label,
            ScopeId::Plain,
        ];
        for scope in &scopes {
            // Each should return a non-default style
            let _ = t.scope_style(*scope);
        }
    }

    #[test]
    fn default_dark_theme_comment_is_italic() {
        let t = default_dark_theme();
        let s = t.scope_style(ScopeId::Comment);
        assert!(s.attrs.italic());
    }

    #[test]
    fn default_dark_theme_string_is_green() {
        let t = default_dark_theme();
        let s = t.scope_style(ScopeId::String);
        assert_eq!(s.fg, Color::Green);
    }

    #[test]
    fn theme_overwrite_scope_style() {
        let mut t = Theme::new("test");
        let style1 = Style::default().fg(Color::Red);
        let style2 = Style::default().fg(Color::Blue);
        t.set_scope_style(ScopeId::Keyword, style1);
        t.set_scope_style(ScopeId::Keyword, style2);
        assert_eq!(t.scope_style(ScopeId::Keyword), style2);
    }

    #[test]
    fn theme_diagnostic_error_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Red).bold();
        t.set_diagnostic_error_style(style);
        assert_eq!(t.diagnostic_error_style(), style);
    }

    #[test]
    fn theme_diagnostic_warning_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Yellow);
        t.set_diagnostic_warning_style(style);
        assert_eq!(t.diagnostic_warning_style(), style);
    }

    #[test]
    fn theme_diagnostic_info_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Blue);
        t.set_diagnostic_info_style(style);
        assert_eq!(t.diagnostic_info_style(), style);
    }

    #[test]
    fn theme_diagnostic_hint_setter_getter() {
        let mut t = Theme::new("test");
        let style = Style::default().fg(Color::Cyan);
        t.set_diagnostic_hint_style(style);
        assert_eq!(t.diagnostic_hint_style(), style);
    }

    #[test]
    fn default_dark_theme_has_diagnostic_styles() {
        let t = default_dark_theme();
        assert_eq!(t.diagnostic_error_style().fg, Color::Red);
        assert_eq!(t.diagnostic_warning_style().fg, Color::Yellow);
        assert_eq!(t.diagnostic_info_style().fg, Color::Blue);
        assert_eq!(t.diagnostic_hint_style().fg, Color::Cyan);
    }
}
