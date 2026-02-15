use std::time::{Duration, Instant};

use crate::command::Command;
use crate::event::{InputEvent, Key, KeyEvent, Modifiers};
use crate::keymap::Keymap;

const CHORD_TIMEOUT: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveResult {
    Command(Command),
    WaitingForMore,
    Unresolved,
}

pub struct KeyResolver {
    keymap: Keymap,
    pending: Vec<KeyEvent>,
    last_key_time: Option<Instant>,
}

impl KeyResolver {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            pending: Vec::new(),
            last_key_time: None,
        }
    }

    pub fn resolve(&mut self, event: InputEvent) -> ResolveResult {
        match event {
            InputEvent::Key(ke) => self.handle_key(ke),
            InputEvent::Paste(ref text) => {
                if let Some(c) = text.chars().next() {
                    ResolveResult::Command(Command::InsertChar(c))
                } else {
                    ResolveResult::Unresolved
                }
            }
            _ => ResolveResult::Unresolved,
        }
    }

    fn handle_key(&mut self, ke: KeyEvent) -> ResolveResult {
        // Check if chord timed out
        if let Some(last) = self.last_key_time {
            if last.elapsed() > CHORD_TIMEOUT {
                self.pending.clear();
            }
        }

        self.pending.push(ke);
        self.last_key_time = Some(Instant::now());

        // Check for exact match
        if let Some(cmd) = self.keymap.resolve(&self.pending) {
            let cmd = cmd.clone();
            self.pending.clear();
            return ResolveResult::Command(cmd);
        }

        // Check for prefix match (multi-key chord)
        if self.keymap.has_prefix(&self.pending) {
            return ResolveResult::WaitingForMore;
        }

        // No match — if single key, try as character insert
        let result = if self.pending.len() == 1 {
            let ke = self.pending[0];
            match ke.key {
                Key::Char(c)
                    if ke.modifiers == Modifiers::NONE || ke.modifiers == Modifiers::SHIFT =>
                {
                    ResolveResult::Command(Command::InsertChar(c))
                }
                _ => ResolveResult::Unresolved,
            }
        } else {
            ResolveResult::Unresolved
        };

        self.pending.clear();
        result
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::MouseEvent;
    use crate::event::MouseKind;
    use crate::keymap::KeymapLayer;

    fn test_keymap() -> Keymap {
        let mut layer = KeymapLayer::new("test");
        layer.bind(vec![KeyEvent::ctrl('s')], Command::Save);
        layer.bind(vec![KeyEvent::ctrl('q')], Command::Quit);
        // Multi-key chord: Ctrl-K, Ctrl-S -> SaveAs
        layer.bind(
            vec![KeyEvent::ctrl('k'), KeyEvent::ctrl('s')],
            Command::SaveAs,
        );
        Keymap::new(layer)
    }

    #[test]
    fn resolve_single_key_binding_returns_command() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('s')));
        assert_eq!(result, ResolveResult::Command(Command::Save));
    }

    #[test]
    fn resolve_unbound_char_returns_insert_char() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Key(KeyEvent::char('a')));
        assert_eq!(result, ResolveResult::Command(Command::InsertChar('a')));
    }

    #[test]
    fn resolve_unbound_modifier_returns_unresolved() {
        let mut resolver = KeyResolver::new(test_keymap());
        let ke = KeyEvent::new(Key::Char('x'), Modifiers::ALT);
        let result = resolver.resolve(InputEvent::Key(ke));
        assert_eq!(result, ResolveResult::Unresolved);
    }

    #[test]
    fn resolve_multi_key_first_key_waits() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('k')));
        assert_eq!(result, ResolveResult::WaitingForMore);
        assert_eq!(resolver.pending_count(), 1);
    }

    #[test]
    fn resolve_multi_key_chord_completes() {
        let mut resolver = KeyResolver::new(test_keymap());

        let r1 = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('k')));
        assert_eq!(r1, ResolveResult::WaitingForMore);

        let r2 = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('s')));
        assert_eq!(r2, ResolveResult::Command(Command::SaveAs));
        assert_eq!(resolver.pending_count(), 0);
    }

    #[test]
    fn resolve_multi_key_wrong_second_key_unresolved() {
        let mut resolver = KeyResolver::new(test_keymap());

        let r1 = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('k')));
        assert_eq!(r1, ResolveResult::WaitingForMore);

        let r2 = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('z')));
        assert_eq!(r2, ResolveResult::Unresolved);
        assert_eq!(resolver.pending_count(), 0);
    }

    #[test]
    fn resolve_paste_returns_insert_char() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Paste("hello".into()));
        assert_eq!(result, ResolveResult::Command(Command::InsertChar('h')));
    }

    #[test]
    fn resolve_empty_paste_returns_unresolved() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Paste(String::new()));
        assert_eq!(result, ResolveResult::Unresolved);
    }

    #[test]
    fn resolve_resize_returns_unresolved() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Resize(80, 24));
        assert_eq!(result, ResolveResult::Unresolved);
    }

    #[test]
    fn resolve_mouse_returns_unresolved() {
        let mut resolver = KeyResolver::new(test_keymap());
        let result = resolver.resolve(InputEvent::Mouse(MouseEvent {
            kind: MouseKind::Down,
            col: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        }));
        assert_eq!(result, ResolveResult::Unresolved);
    }

    #[test]
    fn clear_pending_resets_state() {
        let mut resolver = KeyResolver::new(test_keymap());

        let _ = resolver.resolve(InputEvent::Key(KeyEvent::ctrl('k')));
        assert_eq!(resolver.pending_count(), 1);

        resolver.clear_pending();
        assert_eq!(resolver.pending_count(), 0);
    }

    #[test]
    fn keymap_accessor_returns_keymap() {
        let resolver = KeyResolver::new(test_keymap());
        assert_eq!(resolver.keymap().layer_count(), 1);
    }

    #[test]
    fn keymap_mut_allows_modification() {
        let mut resolver = KeyResolver::new(test_keymap());
        resolver.keymap_mut().push_layer(KeymapLayer::new("extra"));
        assert_eq!(resolver.keymap().layer_count(), 2);
    }

    #[test]
    fn shift_char_treated_as_insert() {
        let mut resolver = KeyResolver::new(test_keymap());
        let ke = KeyEvent::new(Key::Char('A'), Modifiers::SHIFT);
        let result = resolver.resolve(InputEvent::Key(ke));
        assert_eq!(result, ResolveResult::Command(Command::InsertChar('A')));
    }
}
