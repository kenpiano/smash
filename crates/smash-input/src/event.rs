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
    pub fn alt(c: char) -> Self {
        Self {
            key: Key::Char(c),
            modifiers: Modifiers::ALT,
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

/// Map a macOS Option-produced Unicode character back to its original
/// ASCII key. On US-QWERTY macOS keyboards, pressing Option+key
/// produces a special Unicode character (e.g. Option+f → ƒ). This
/// table maps those characters back so they can be treated as Alt+key.
///
/// Returns `Some((original_char, is_shifted))` if the character is a
/// known macOS Option mapping, `None` otherwise.
fn macos_option_char_to_key(c: char) -> Option<(char, bool)> {
    // US-QWERTY Option key mappings (unshifted)
    match c {
        'å' => Some(('a', false)),
        '∫' => Some(('b', false)),
        'ç' => Some(('c', false)),
        '∂' => Some(('d', false)),
        'ƒ' => Some(('f', false)),
        '©' => Some(('g', false)),
        '˙' => Some(('h', false)),
        '∆' => Some(('j', false)),
        '˚' => Some(('k', false)),
        '¬' => Some(('l', false)),
        'µ' => Some(('m', false)),
        'ø' => Some(('o', false)),
        'π' => Some(('p', false)),
        'œ' => Some(('q', false)),
        '®' => Some(('r', false)),
        'ß' => Some(('s', false)),
        '†' => Some(('t', false)),
        '√' => Some(('v', false)),
        '∑' => Some(('w', false)),
        '≈' => Some(('x', false)),
        '¥' => Some(('y', false)),
        'Ω' => Some(('z', false)),
        // Punctuation
        '≤' => Some((',', false)),
        '≥' => Some(('.', false)),
        '÷' => Some(('/', false)),
        '˘' => Some(('>', false)),
        '¯' => Some(('<', false)),
        '¿' => Some(('?', false)),
        // Option+Shift variants
        'Å' => Some(('a', true)),
        'ı' => Some(('b', true)),
        'Ç' => Some(('c', true)),
        'Î' => Some(('d', true)),
        'Ï' => Some(('f', true)),
        '˝' => Some(('g', true)),
        'Ó' => Some(('h', true)),
        'Ô' => Some(('j', true)),
        '\u{F8FF}' => Some(('K', true)), // Apple logo
        'Ò' => Some(('l', true)),
        'Â' => Some(('m', true)),
        'Ø' => Some(('o', true)),
        '∏' => Some(('p', true)),
        'Œ' => Some(('q', true)),
        '‰' => Some(('r', true)),
        'Í' => Some(('s', true)),
        'ˇ' => Some(('t', true)),
        '◊' => Some(('v', true)),
        '„' => Some(('w', true)),
        '˛' => Some(('x', true)),
        'Á' => Some(('y', true)),
        '¸' => Some(('z', true)),
        _ => None,
    }
}

/// Normalize a [`KeyEvent`] for macOS Option-as-Alt behaviour.
///
/// When macOS Terminal.app (or similar) sends a Unicode character
/// produced by the Option key, this function maps it back to the
/// original key with the [`Modifiers::ALT`] flag set. This allows
/// Alt-based keybindings (e.g. Emacs Alt-f for word-forward) to
/// work correctly on macOS without requiring terminal configuration.
///
/// Call this on every [`KeyEvent`] *before* passing it to the
/// key resolver. It is a no-op on non-macOS platforms.
pub fn normalize_macos_option_key(mut event: KeyEvent) -> KeyEvent {
    // Only normalize plain characters (no existing modifiers) or
    // characters that already have SHIFT but nothing else.
    if event.modifiers != Modifiers::NONE && event.modifiers != Modifiers::SHIFT {
        return event;
    }
    if let Key::Char(c) = event.key {
        if let Some((original, shifted)) = macos_option_char_to_key(c) {
            event.key = Key::Char(original);
            event.modifiers = Modifiers::ALT;
            if shifted {
                event.modifiers = event.modifiers | Modifiers::SHIFT;
            }
        }
    }
    event
}

/// Convert crossterm events to our normalized InputEvent.
///
/// On Windows, crossterm emits separate `Press`, `Release`, and `Repeat`
/// events for every key stroke.  We process `Press` and `Repeat` events
/// (the latter is needed for key-hold continuous actions such as cursor
/// movement) but discard `Release` events to avoid double-firing.
pub fn from_crossterm(event: crossterm::event::Event) -> Option<InputEvent> {
    match event {
        crossterm::event::Event::Key(ke) => {
            // Filter: accept Press and Repeat, discard Release.
            // Repeat events fire when a key is held down (Windows) and are
            // required for continuous movement, text insertion, etc.
            if ke.kind == crossterm::event::KeyEventKind::Release {
                return None;
            }
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

    // ── KeyEvent::alt() constructor ────────────────────────────

    #[test]
    fn key_event_alt_constructor_sets_alt() {
        let ke = KeyEvent::alt('f');
        assert_eq!(ke.key, Key::Char('f'));
        assert!(ke.modifiers.alt());
        assert!(!ke.modifiers.ctrl());
        assert!(!ke.modifiers.shift());
    }

    // ── macOS Option key normalization ────────────────────────

    #[test]
    fn normalize_option_f_to_alt_f() {
        // Option+f on macOS Terminal.app produces 'ƒ' (U+0192)
        let input = KeyEvent::char('\u{0192}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('f'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_b_to_alt_b() {
        // Option+b → '∫' (U+222B)
        let input = KeyEvent::char('\u{222B}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('b'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_v_to_alt_v() {
        // Option+v → '√' (U+221A)
        let input = KeyEvent::char('\u{221A}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('v'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_x_to_alt_x() {
        // Option+x → '≈' (U+2248)
        let input = KeyEvent::char('\u{2248}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('x'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_less_than() {
        // Option+< → '≤' (U+2264)
        let input = KeyEvent::char('\u{2264}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char(','));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_greater_than() {
        // Option+> → '≥' (U+2265)
        let input = KeyEvent::char('\u{2265}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('.'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_slash() {
        // Option+/ → '÷' (U+00F7)
        let input = KeyEvent::char('\u{00F7}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('/'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_question() {
        // Option+? → '¿' (U+00BF)
        let input = KeyEvent::char('\u{00BF}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('?'));
        assert!(result.modifiers.alt());
    }

    #[test]
    fn normalize_option_shift_a() {
        // Option+Shift+a → 'Å' (U+00C5)
        let input = KeyEvent::char('\u{00C5}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('a'));
        assert!(result.modifiers.alt());
        assert!(result.modifiers.shift());
    }

    #[test]
    fn normalize_option_shift_p() {
        // Option+Shift+p → '∏' (U+220F)
        let input = KeyEvent::char('\u{220F}');
        let result = normalize_macos_option_key(input);
        assert_eq!(result.key, Key::Char('p'));
        assert!(result.modifiers.alt());
        assert!(result.modifiers.shift());
    }

    #[test]
    fn normalize_plain_ascii_unchanged() {
        let input = KeyEvent::char('a');
        let result = normalize_macos_option_key(input);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_ctrl_char_unchanged() {
        // Characters with existing Ctrl modifier should not be touched
        let input = KeyEvent::ctrl('f');
        let result = normalize_macos_option_key(input);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_already_alt_unchanged() {
        // Character already flagged as Alt should not be double-modified
        let input = KeyEvent::alt('f');
        let result = normalize_macos_option_key(input);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_non_key_event_unchanged() {
        // Non-Char keys should pass through
        let input = KeyEvent::new(Key::Enter, Modifiers::NONE);
        let result = normalize_macos_option_key(input);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_unknown_unicode_unchanged() {
        // A Unicode char not in the macOS Option table should pass through
        let input = KeyEvent::char('\u{1F600}'); // 😀
        let result = normalize_macos_option_key(input);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_all_alpha_option_keys() {
        // Verify all lowercase alpha Option mappings
        let pairs = [
            ('\u{00e5}', 'a'), // å
            ('\u{222b}', 'b'), // ∫
            ('\u{00e7}', 'c'), // ç
            ('\u{2202}', 'd'), // ∂
            ('\u{0192}', 'f'), // ƒ
            ('\u{00a9}', 'g'), // ©
            ('\u{02d9}', 'h'), // ˙
            ('\u{2206}', 'j'), // ∆
            ('\u{02da}', 'k'), // ˚
            ('\u{00ac}', 'l'), // ¬
            ('\u{00b5}', 'm'), // µ
            ('\u{00f8}', 'o'), // ø
            ('\u{03c0}', 'p'), // π
            ('\u{0153}', 'q'), // œ
            ('\u{00ae}', 'r'), // ®
            ('\u{00df}', 's'), // ß
            ('\u{2020}', 't'), // †
            ('\u{221a}', 'v'), // √
            ('\u{2211}', 'w'), // ∑
            ('\u{2248}', 'x'), // ≈
            ('\u{00a5}', 'y'), // ¥
            ('\u{03a9}', 'z'), // Ω
        ];
        for (unicode, expected) in pairs {
            let result = normalize_macos_option_key(KeyEvent::char(unicode));
            assert_eq!(
                result.key,
                Key::Char(expected),
                "Option+{expected} mapping failed for U+{:04X}",
                unicode as u32
            );
            assert!(result.modifiers.alt(), "ALT not set for Option+{expected}");
            assert!(
                !result.modifiers.shift(),
                "SHIFT should not be set for Option+{expected}"
            );
        }
    }

    // =================================================================
    // Windows key-event kind filtering
    // =================================================================

    #[test]
    fn from_crossterm_key_release_is_ignored() {
        // Simulate a Release event (Windows sends these alongside Press)
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('a'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(
            from_crossterm(ct_event),
            None,
            "Release events must be filtered out to prevent double-firing on Windows"
        );
    }

    #[test]
    fn from_crossterm_key_repeat_is_accepted() {
        // Repeat events fire when a key is held down (e.g. holding an arrow
        // key for continuous cursor movement).  They must be processed.
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('a'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Repeat,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(
            from_crossterm(ct_event),
            Some(InputEvent::Key(KeyEvent::char('a'))),
            "Repeat events must be processed for key-hold actions"
        );
    }

    #[test]
    fn from_crossterm_key_press_is_accepted() {
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('a'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(
            from_crossterm(ct_event),
            Some(InputEvent::Key(KeyEvent::char('a'))),
            "Press events must be processed normally"
        );
    }
}
