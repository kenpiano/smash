use smash_input::{Key, KeyEvent};

/// Translate a `KeyEvent` into the corresponding terminal escape sequence bytes.
pub fn key_to_escape_sequence(event: &KeyEvent) -> Vec<u8> {
    let mods = event.modifiers;

    // Ctrl + key combinations
    if mods.ctrl() && !mods.alt() && !mods.shift() {
        if let Key::Char(c) = event.key {
            return match c {
                'a'..='z' => vec![c as u8 - b'a' + 1],
                '@' => vec![0x00],
                '[' => vec![0x1b],
                '\\' => vec![0x1c],
                ']' => vec![0x1d],
                '^' => vec![0x1e],
                '_' => vec![0x1f],
                _ => vec![],
            };
        }
    }

    // Alt + key — send ESC prefix then the character
    if mods.alt() && !mods.ctrl() {
        if let Key::Char(c) = event.key {
            let mut seq = vec![0x1b];
            let ch = if mods.shift() {
                c.to_ascii_uppercase()
            } else {
                c
            };
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            seq.extend_from_slice(encoded.as_bytes());
            return seq;
        }
    }

    match event.key {
        Key::Char(c) => {
            let ch = if mods.shift() {
                c.to_ascii_uppercase()
            } else {
                c
            };
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            encoded.as_bytes().to_vec()
        }
        Key::Enter => vec![0x0d],
        Key::Tab => {
            if mods.shift() {
                vec![0x1b, b'[', b'Z'] // Shift-Tab (back tab)
            } else {
                vec![0x09]
            }
        }
        Key::Backspace => vec![0x7f],
        Key::Delete => vec![0x1b, b'[', b'3', b'~'],
        Key::Esc => vec![0x1b],
        Key::Up => vec![0x1b, b'[', b'A'],
        Key::Down => vec![0x1b, b'[', b'B'],
        Key::Right => vec![0x1b, b'[', b'C'],
        Key::Left => vec![0x1b, b'[', b'D'],
        Key::Home => vec![0x1b, b'[', b'H'],
        Key::End => vec![0x1b, b'[', b'F'],
        Key::PageUp => vec![0x1b, b'[', b'5', b'~'],
        Key::PageDown => vec![0x1b, b'[', b'6', b'~'],
        Key::F(n) => function_key_sequence(n),
        Key::Null => vec![0x00],
    }
}

/// Generate the escape sequence for a function key.
fn function_key_sequence(n: u8) -> Vec<u8> {
    match n {
        1 => vec![0x1b, b'O', b'P'],
        2 => vec![0x1b, b'O', b'Q'],
        3 => vec![0x1b, b'O', b'R'],
        4 => vec![0x1b, b'O', b'S'],
        5 => vec![0x1b, b'[', b'1', b'5', b'~'],
        6 => vec![0x1b, b'[', b'1', b'7', b'~'],
        7 => vec![0x1b, b'[', b'1', b'8', b'~'],
        8 => vec![0x1b, b'[', b'1', b'9', b'~'],
        9 => vec![0x1b, b'[', b'2', b'0', b'~'],
        10 => vec![0x1b, b'[', b'2', b'1', b'~'],
        11 => vec![0x1b, b'[', b'2', b'3', b'~'],
        12 => vec![0x1b, b'[', b'2', b'4', b'~'],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smash_input::Modifiers;

    #[test]
    fn input_plain_char() {
        let event = KeyEvent::new(Key::Char('a'), Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x61]);
    }

    #[test]
    fn input_enter() {
        let event = KeyEvent::new(Key::Enter, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x0d]);
    }

    #[test]
    fn input_arrow_up() {
        let event = KeyEvent::new(Key::Up, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'A']);
    }

    #[test]
    fn input_ctrl_c() {
        let event = KeyEvent::new(Key::Char('c'), Modifiers::CTRL);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x03]);
    }

    #[test]
    fn input_function_key_f1() {
        let event = KeyEvent::new(Key::F(1), Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'O', b'P']);
    }

    #[test]
    fn input_backspace() {
        let event = KeyEvent::new(Key::Backspace, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x7f]);
    }

    #[test]
    fn input_delete() {
        let event = KeyEvent::new(Key::Delete, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'3', b'~']);
    }

    #[test]
    fn input_escape() {
        let event = KeyEvent::new(Key::Esc, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b]);
    }

    #[test]
    fn input_home() {
        let event = KeyEvent::new(Key::Home, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'H']);
    }

    #[test]
    fn input_end() {
        let event = KeyEvent::new(Key::End, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'F']);
    }

    #[test]
    fn input_page_up() {
        let event = KeyEvent::new(Key::PageUp, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'5', b'~']);
    }

    #[test]
    fn input_tab() {
        let event = KeyEvent::new(Key::Tab, Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x09]);
    }

    #[test]
    fn input_shift_tab() {
        let event = KeyEvent::new(Key::Tab, Modifiers::SHIFT);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'Z']);
    }

    #[test]
    fn input_ctrl_a() {
        let event = KeyEvent::new(Key::Char('a'), Modifiers::CTRL);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x01]);
    }

    #[test]
    fn input_ctrl_z() {
        let event = KeyEvent::new(Key::Char('z'), Modifiers::CTRL);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1a]);
    }

    #[test]
    fn input_alt_a() {
        let event = KeyEvent::new(Key::Char('a'), Modifiers::ALT);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'a']);
    }

    #[test]
    fn input_function_key_f5() {
        let event = KeyEvent::new(Key::F(5), Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'1', b'5', b'~']);
    }

    #[test]
    fn input_function_key_f12() {
        let event = KeyEvent::new(Key::F(12), Modifiers::NONE);
        let seq = key_to_escape_sequence(&event);
        assert_eq!(seq, vec![0x1b, b'[', b'2', b'4', b'~']);
    }
}
