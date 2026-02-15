use std::fmt;

/// Modifier keys as bitflags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1);
    pub const ALT: Self = Self(2);
    pub const SHIFT: Self = Self(4);
    pub const SUPER: Self = Self(8);

    pub fn ctrl(self) -> bool {
        self.0 & 1 != 0
    }
    pub fn alt(self) -> bool {
        self.0 & 2 != 0
    }
    pub fn shift(self) -> bool {
        self.0 & 4 != 0
    }
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
    pub fn char(c: char) -> Self {
        Self {
            key: Key::Char(c),
            modifiers: Modifiers::NONE,
        }
    }
    pub fn ctrl(c: char) -> Self {
        Self {
            key: Key::Char(c),
            modifiers: Modifiers::CTRL,
        }
    }
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.ctrl() {
            write!(f, "Ctrl-")?;
        }
        if self.modifiers.alt() {
            write!(f, "Alt-")?;
        }
        if self.modifiers.shift() {
            write!(f, "Shift-")?;
        }
        match self.key {
            Key::Char(c) => write!(f, "{}", c.to_uppercase()),
            Key::Enter => write!(f, "Enter"),
            Key::Esc => write!(f, "Esc"),
            Key::Tab => write!(f, "Tab"),
            Key::Backspace => write!(f, "Backspace"),
            Key::Delete => write!(f, "Delete"),
            Key::Left => write!(f, "Left"),
            Key::Right => write!(f, "Right"),
            Key::Up => write!(f, "Up"),
            Key::Down => write!(f, "Down"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PageUp"),
            Key::PageDown => write!(f, "PageDown"),
            Key::F(n) => write!(f, "F{}", n),
            Key::Null => write!(f, "Null"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseKind,
    pub col: u16,
    pub row: u16,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseKind {
    Down,
    Up,
    Drag,
    ScrollUp,
    ScrollDown,
    Move,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Paste(String),
}

/// Convert crossterm events to our normalized InputEvent
pub fn from_crossterm(event: crossterm::event::Event) -> Option<InputEvent> {
    match event {
        crossterm::event::Event::Key(ke) => {
            let modifiers = convert_modifiers(ke.modifiers);
            let key = convert_key(ke.code)?;
            Some(InputEvent::Key(KeyEvent::new(key, modifiers)))
        }
        crossterm::event::Event::Resize(w, h) => Some(InputEvent::Resize(w, h)),
        crossterm::event::Event::Paste(s) => Some(InputEvent::Paste(s)),
        crossterm::event::Event::Mouse(me) => {
            let kind = match me.kind {
                crossterm::event::MouseEventKind::Down(_) => MouseKind::Down,
                crossterm::event::MouseEventKind::Up(_) => MouseKind::Up,
                crossterm::event::MouseEventKind::Drag(_) => MouseKind::Drag,
                crossterm::event::MouseEventKind::ScrollUp => MouseKind::ScrollUp,
                crossterm::event::MouseEventKind::ScrollDown => MouseKind::ScrollDown,
                crossterm::event::MouseEventKind::Moved => MouseKind::Move,
                _ => return None,
            };
            Some(InputEvent::Mouse(MouseEvent {
                kind,
                col: me.column,
                row: me.row,
                modifiers: convert_modifiers(me.modifiers),
            }))
        }
        _ => None,
    }
}

fn convert_modifiers(m: crossterm::event::KeyModifiers) -> Modifiers {
    let mut mods = Modifiers::NONE;
    if m.contains(crossterm::event::KeyModifiers::CONTROL) {
        mods = mods | Modifiers::CTRL;
    }
    if m.contains(crossterm::event::KeyModifiers::ALT) {
        mods = mods | Modifiers::ALT;
    }
    if m.contains(crossterm::event::KeyModifiers::SHIFT) {
        mods = mods | Modifiers::SHIFT;
    }
    if m.contains(crossterm::event::KeyModifiers::SUPER) {
        mods = mods | Modifiers::SUPER;
    }
    mods
}

fn convert_key(code: crossterm::event::KeyCode) -> Option<Key> {
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Char(c) => Some(Key::Char(c)),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Tab | KeyCode::BackTab => Some(Key::Tab),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::F(n) => Some(Key::F(n)),
        KeyCode::Null => Some(Key::Null),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_char_constructor_no_modifiers() {
        let ke = KeyEvent::char('a');
        assert_eq!(ke.key, Key::Char('a'));
        assert_eq!(ke.modifiers, Modifiers::NONE);
    }

    #[test]
    fn key_event_ctrl_constructor_sets_ctrl() {
        let ke = KeyEvent::ctrl('s');
        assert_eq!(ke.key, Key::Char('s'));
        assert!(ke.modifiers.ctrl());
        assert!(!ke.modifiers.alt());
        assert!(!ke.modifiers.shift());
    }

    #[test]
    fn modifiers_bitor_combines_flags() {
        let mods = Modifiers::CTRL | Modifiers::ALT;
        assert!(mods.ctrl());
        assert!(mods.alt());
        assert!(!mods.shift());
    }

    #[test]
    fn modifiers_contains_checks_subset() {
        let mods = Modifiers::CTRL | Modifiers::ALT;
        assert!(mods.contains(Modifiers::CTRL));
        assert!(mods.contains(Modifiers::ALT));
        assert!(!mods.contains(Modifiers::SHIFT));
        assert!(mods.contains(Modifiers::CTRL | Modifiers::ALT));
    }

    #[test]
    fn modifiers_none_has_no_flags() {
        let mods = Modifiers::NONE;
        assert!(!mods.ctrl());
        assert!(!mods.alt());
        assert!(!mods.shift());
    }

    #[test]
    fn display_plain_char() {
        let ke = KeyEvent::char('a');
        assert_eq!(ke.to_string(), "A");
    }

    #[test]
    fn display_ctrl_char() {
        let ke = KeyEvent::ctrl('s');
        assert_eq!(ke.to_string(), "Ctrl-S");
    }

    #[test]
    fn display_ctrl_alt_char() {
        let ke = KeyEvent::new(Key::Char('x'), Modifiers::CTRL | Modifiers::ALT);
        assert_eq!(ke.to_string(), "Ctrl-Alt-X");
    }

    #[test]
    fn display_special_keys() {
        assert_eq!(
            KeyEvent::new(Key::Enter, Modifiers::NONE).to_string(),
            "Enter"
        );
        assert_eq!(KeyEvent::new(Key::Esc, Modifiers::NONE).to_string(), "Esc");
        assert_eq!(KeyEvent::new(Key::F(5), Modifiers::NONE).to_string(), "F5");
        assert_eq!(
            KeyEvent::new(Key::PageUp, Modifiers::NONE).to_string(),
            "PageUp"
        );
    }

    #[test]
    fn display_shift_tab() {
        let ke = KeyEvent::new(Key::Tab, Modifiers::SHIFT);
        assert_eq!(ke.to_string(), "Shift-Tab");
    }

    #[test]
    fn input_event_key_variant() {
        let evt = InputEvent::Key(KeyEvent::char('z'));
        match evt {
            InputEvent::Key(ke) => {
                assert_eq!(ke.key, Key::Char('z'));
            }
            _ => panic!("expected Key variant"),
        }
    }

    #[test]
    fn input_event_resize_variant() {
        let evt = InputEvent::Resize(80, 24);
        assert_eq!(evt, InputEvent::Resize(80, 24));
    }

    #[test]
    fn input_event_paste_variant() {
        let evt = InputEvent::Paste("hello".into());
        assert_eq!(evt, InputEvent::Paste("hello".into()));
    }

    #[test]
    fn input_event_mouse_variant() {
        let evt = InputEvent::Mouse(MouseEvent {
            kind: MouseKind::Down,
            col: 10,
            row: 5,
            modifiers: Modifiers::NONE,
        });
        match evt {
            InputEvent::Mouse(me) => {
                assert_eq!(me.kind, MouseKind::Down);
                assert_eq!(me.col, 10);
                assert_eq!(me.row, 5);
            }
            _ => panic!("expected Mouse variant"),
        }
    }

    #[test]
    fn key_event_new_constructor() {
        let ke = KeyEvent::new(Key::Home, Modifiers::CTRL);
        assert_eq!(ke.key, Key::Home);
        assert!(ke.modifiers.ctrl());
    }

    #[test]
    fn key_event_equality() {
        let a = KeyEvent::ctrl('s');
        let b = KeyEvent::new(Key::Char('s'), Modifiers::CTRL);
        assert_eq!(a, b);
    }

    // Tests for crossterm conversion functions

    #[test]
    fn convert_modifiers_none() {
        let m = crossterm::event::KeyModifiers::NONE;
        let result = convert_modifiers(m);
        assert_eq!(result, Modifiers::NONE);
    }

    #[test]
    fn convert_modifiers_ctrl() {
        let m = crossterm::event::KeyModifiers::CONTROL;
        let result = convert_modifiers(m);
        assert!(result.ctrl());
        assert!(!result.alt());
    }

    #[test]
    fn convert_modifiers_alt() {
        let m = crossterm::event::KeyModifiers::ALT;
        let result = convert_modifiers(m);
        assert!(result.alt());
    }

    #[test]
    fn convert_modifiers_shift() {
        let m = crossterm::event::KeyModifiers::SHIFT;
        let result = convert_modifiers(m);
        assert!(result.shift());
    }

    #[test]
    fn convert_modifiers_combined() {
        let m = crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT;
        let result = convert_modifiers(m);
        assert!(result.ctrl());
        assert!(result.alt());
    }

    #[test]
    fn convert_modifiers_super() {
        let m = crossterm::event::KeyModifiers::SUPER;
        let result = convert_modifiers(m);
        assert!(result.contains(Modifiers::SUPER));
    }

    #[test]
    fn convert_key_char() {
        let k = crossterm::event::KeyCode::Char('x');
        assert_eq!(convert_key(k), Some(Key::Char('x')));
    }

    #[test]
    fn convert_key_enter() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Enter),
            Some(Key::Enter)
        );
    }

    #[test]
    fn convert_key_esc() {
        assert_eq!(convert_key(crossterm::event::KeyCode::Esc), Some(Key::Esc));
    }

    #[test]
    fn convert_key_tab() {
        assert_eq!(convert_key(crossterm::event::KeyCode::Tab), Some(Key::Tab));
    }

    #[test]
    fn convert_key_backspace() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Backspace),
            Some(Key::Backspace)
        );
    }

    #[test]
    fn convert_key_delete() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Delete),
            Some(Key::Delete)
        );
    }

    #[test]
    fn convert_key_arrows() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Left),
            Some(Key::Left)
        );
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Right),
            Some(Key::Right)
        );
        assert_eq!(convert_key(crossterm::event::KeyCode::Up), Some(Key::Up));
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Down),
            Some(Key::Down)
        );
    }

    #[test]
    fn convert_key_home_end() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Home),
            Some(Key::Home)
        );
        assert_eq!(convert_key(crossterm::event::KeyCode::End), Some(Key::End));
    }

    #[test]
    fn convert_key_page_up_down() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::PageUp),
            Some(Key::PageUp)
        );
        assert_eq!(
            convert_key(crossterm::event::KeyCode::PageDown),
            Some(Key::PageDown)
        );
    }

    #[test]
    fn convert_key_function_keys() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::F(1)),
            Some(Key::F(1))
        );
        assert_eq!(
            convert_key(crossterm::event::KeyCode::F(12)),
            Some(Key::F(12))
        );
    }

    #[test]
    fn convert_key_null() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::Null),
            Some(Key::Null)
        );
    }

    #[test]
    fn convert_key_backtab() {
        assert_eq!(
            convert_key(crossterm::event::KeyCode::BackTab),
            Some(Key::Tab)
        );
    }

    #[test]
    fn from_crossterm_key_event() {
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = from_crossterm(ct_event);
        assert_eq!(result, Some(InputEvent::Key(KeyEvent::char('a'))));
    }

    #[test]
    fn from_crossterm_ctrl_key() {
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('s'),
            crossterm::event::KeyModifiers::CONTROL,
        ));
        let result = from_crossterm(ct_event);
        assert_eq!(result, Some(InputEvent::Key(KeyEvent::ctrl('s'))));
    }

    #[test]
    fn from_crossterm_resize() {
        let ct_event = crossterm::event::Event::Resize(120, 40);
        let result = from_crossterm(ct_event);
        assert_eq!(result, Some(InputEvent::Resize(120, 40)));
    }

    #[test]
    fn from_crossterm_paste() {
        let ct_event = crossterm::event::Event::Paste("pasted text".into());
        let result = from_crossterm(ct_event);
        assert_eq!(result, Some(InputEvent::Paste("pasted text".into())));
    }

    #[test]
    fn from_crossterm_mouse_scroll_up() {
        let ct_event = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 5,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = from_crossterm(ct_event);
        match result {
            Some(InputEvent::Mouse(me)) => {
                assert_eq!(me.kind, MouseKind::ScrollUp);
                assert_eq!(me.col, 5);
                assert_eq!(me.row, 10);
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn from_crossterm_mouse_scroll_down() {
        let ct_event = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = from_crossterm(ct_event);
        match result {
            Some(InputEvent::Mouse(me)) => {
                assert_eq!(me.kind, MouseKind::ScrollDown);
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn from_crossterm_mouse_moved() {
        let ct_event = crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Moved,
            column: 3,
            row: 7,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let result = from_crossterm(ct_event);
        match result {
            Some(InputEvent::Mouse(me)) => {
                assert_eq!(me.kind, MouseKind::Move);
            }
            _ => panic!("expected Mouse event"),
        }
    }

    #[test]
    fn from_crossterm_focus_gained_returns_none() {
        let ct_event = crossterm::event::Event::FocusGained;
        let result = from_crossterm(ct_event);
        assert_eq!(result, None);
    }

    #[test]
    fn from_crossterm_focus_lost_returns_none() {
        let ct_event = crossterm::event::Event::FocusLost;
        let result = from_crossterm(ct_event);
        assert_eq!(result, None);
    }

    #[test]
    fn display_remaining_special_keys() {
        assert_eq!(
            KeyEvent::new(Key::Backspace, Modifiers::NONE).to_string(),
            "Backspace"
        );
        assert_eq!(
            KeyEvent::new(Key::Delete, Modifiers::NONE).to_string(),
            "Delete"
        );
        assert_eq!(
            KeyEvent::new(Key::Left, Modifiers::NONE).to_string(),
            "Left"
        );
        assert_eq!(
            KeyEvent::new(Key::Right, Modifiers::NONE).to_string(),
            "Right"
        );
        assert_eq!(KeyEvent::new(Key::Up, Modifiers::NONE).to_string(), "Up");
        assert_eq!(
            KeyEvent::new(Key::Down, Modifiers::NONE).to_string(),
            "Down"
        );
        assert_eq!(
            KeyEvent::new(Key::Home, Modifiers::NONE).to_string(),
            "Home"
        );
        assert_eq!(KeyEvent::new(Key::End, Modifiers::NONE).to_string(), "End");
        assert_eq!(
            KeyEvent::new(Key::PageDown, Modifiers::NONE).to_string(),
            "PageDown"
        );
        assert_eq!(
            KeyEvent::new(Key::Null, Modifiers::NONE).to_string(),
            "Null"
        );
    }
}
