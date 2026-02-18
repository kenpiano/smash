use crate::command::Command;
use crate::event::{Key, KeyEvent, Modifiers};
use crate::keymap::KeymapLayer;

pub fn create_default_keymap() -> KeymapLayer {
    let mut layer = KeymapLayer::new("default");

    // File ops
    layer.bind(vec![KeyEvent::ctrl('s')], Command::Save);
    layer.bind(vec![KeyEvent::ctrl('q')], Command::Quit);
    layer.bind(vec![KeyEvent::ctrl('o')], Command::Open);
    layer.bind(vec![KeyEvent::ctrl('w')], Command::ClosePane);

    // Edit
    layer.bind(vec![KeyEvent::ctrl('z')], Command::Undo);
    layer.bind(
        vec![KeyEvent::new(
            Key::Char('Z'),
            Modifiers::CTRL | Modifiers::SHIFT,
        )],
        Command::Redo,
    );

    // Search
    layer.bind(vec![KeyEvent::ctrl('f')], Command::Find);
    layer.bind(vec![KeyEvent::ctrl('h')], Command::FindReplace);
    layer.bind(vec![KeyEvent::ctrl('g')], Command::GoToLine);
    layer.bind(vec![KeyEvent::ctrl('p')], Command::OpenCommandPalette);
    layer.bind(vec![KeyEvent::ctrl('d')], Command::AddCursorBelow);
    layer.bind(
        vec![KeyEvent::new(Key::F(3), Modifiers::NONE)],
        Command::FindNext,
    );
    layer.bind(
        vec![KeyEvent::new(Key::F(3), Modifiers::SHIFT)],
        Command::FindPrev,
    );
    layer.bind(vec![KeyEvent::ctrl('n')], Command::FindNext);
    layer.bind(
        vec![KeyEvent::new(
            Key::Char('N'),
            Modifiers::CTRL | Modifiers::SHIFT,
        )],
        Command::FindPrev,
    );

    // Navigation (no modifier)
    layer.bind(
        vec![KeyEvent::new(Key::Enter, Modifiers::NONE)],
        Command::InsertNewline,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Tab, Modifiers::NONE)],
        Command::InsertChar('\t'),
    );
    layer.bind(
        vec![KeyEvent::new(Key::Backspace, Modifiers::NONE)],
        Command::DeleteBackward,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Delete, Modifiers::NONE)],
        Command::DeleteForward,
    );

    layer.bind(
        vec![KeyEvent::new(Key::Left, Modifiers::NONE)],
        Command::MoveLeft,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Right, Modifiers::NONE)],
        Command::MoveRight,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Up, Modifiers::NONE)],
        Command::MoveUp,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Down, Modifiers::NONE)],
        Command::MoveDown,
    );

    layer.bind(
        vec![KeyEvent::new(Key::Home, Modifiers::NONE)],
        Command::MoveLineStart,
    );
    layer.bind(
        vec![KeyEvent::new(Key::End, Modifiers::NONE)],
        Command::MoveLineEnd,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Home, Modifiers::CTRL)],
        Command::MoveBufferStart,
    );
    layer.bind(
        vec![KeyEvent::new(Key::End, Modifiers::CTRL)],
        Command::MoveBufferEnd,
    );

    layer.bind(
        vec![KeyEvent::new(Key::Left, Modifiers::CTRL)],
        Command::MoveWordLeft,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Right, Modifiers::CTRL)],
        Command::MoveWordRight,
    );

    layer.bind(
        vec![KeyEvent::new(Key::PageUp, Modifiers::NONE)],
        Command::PageUp,
    );
    layer.bind(
        vec![KeyEvent::new(Key::PageDown, Modifiers::NONE)],
        Command::PageDown,
    );

    // Select all
    layer.bind(vec![KeyEvent::ctrl('a')], Command::SelectAll);

    // Terminal
    layer.bind(
        vec![KeyEvent::new(Key::Char('\\'), Modifiers::CTRL)],
        Command::ToggleTerminal,
    );

    // LSP
    layer.bind(
        vec![KeyEvent::new(Key::F(12), Modifiers::NONE)],
        Command::LspGotoDefinition,
    );
    layer.bind(
        vec![KeyEvent::new(Key::F(12), Modifiers::SHIFT)],
        Command::LspFindReferences,
    );
    layer.bind(
        vec![KeyEvent::new(Key::F(2), Modifiers::NONE)],
        Command::LspRename,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char(' '), Modifiers::CTRL)],
        Command::LspCompletion,
    );
    layer.bind(
        vec![KeyEvent::new(
            Key::Char('F'),
            Modifiers::CTRL | Modifiers::SHIFT,
        )],
        Command::LspFormat,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('.'), Modifiers::CTRL)],
        Command::LspCodeAction,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('K'), Modifiers::CTRL)],
        Command::LspHover,
    );
    layer.bind(
        vec![KeyEvent::new(Key::F(8), Modifiers::NONE)],
        Command::LspDiagnosticNext,
    );
    layer.bind(
        vec![KeyEvent::new(Key::F(8), Modifiers::SHIFT)],
        Command::LspDiagnosticPrev,
    );

    // Jump navigation
    layer.bind(
        vec![KeyEvent::new(Key::Char('O'), Modifiers::CTRL)],
        Command::JumpBack,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('I'), Modifiers::CTRL)],
        Command::JumpForward,
    );

    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_ctrl_s_is_save() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('s')];
        assert_eq!(layer.get(&seq), Some(&Command::Save));
    }

    #[test]
    fn default_keymap_ctrl_q_is_quit() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('q')];
        assert_eq!(layer.get(&seq), Some(&Command::Quit));
    }

    #[test]
    fn default_keymap_ctrl_z_is_undo() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('z')];
        assert_eq!(layer.get(&seq), Some(&Command::Undo));
    }

    #[test]
    fn default_keymap_ctrl_shift_z_is_redo() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(
            Key::Char('Z'),
            Modifiers::CTRL | Modifiers::SHIFT,
        )];
        assert_eq!(layer.get(&seq), Some(&Command::Redo));
    }

    #[test]
    fn default_keymap_arrow_keys_move() {
        let layer = create_default_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Left, Modifiers::NONE)]),
            Some(&Command::MoveLeft)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Right, Modifiers::NONE)]),
            Some(&Command::MoveRight)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Up, Modifiers::NONE)]),
            Some(&Command::MoveUp)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Down, Modifiers::NONE)]),
            Some(&Command::MoveDown)
        );
    }

    #[test]
    fn default_keymap_enter_inserts_newline() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(Key::Enter, Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::InsertNewline));
    }

    #[test]
    fn default_keymap_backspace_deletes_backward() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(Key::Backspace, Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::DeleteBackward));
    }

    #[test]
    fn default_keymap_ctrl_f_is_find() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('f')];
        assert_eq!(layer.get(&seq), Some(&Command::Find));
    }

    #[test]
    fn default_keymap_ctrl_a_is_select_all() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('a')];
        assert_eq!(layer.get(&seq), Some(&Command::SelectAll));
    }

    #[test]
    fn default_keymap_unknown_binding_returns_none() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(Key::F(11), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), None);
    }

    #[test]
    fn default_keymap_page_up_down() {
        let layer = create_default_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageUp, Modifiers::NONE)]),
            Some(&Command::PageUp)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageDown, Modifiers::NONE)]),
            Some(&Command::PageDown)
        );
    }

    #[test]
    fn default_keymap_ctrl_word_movement() {
        let layer = create_default_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Left, Modifiers::CTRL)]),
            Some(&Command::MoveWordLeft)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Right, Modifiers::CTRL)]),
            Some(&Command::MoveWordRight)
        );
    }

    #[test]
    fn default_keymap_home_end() {
        let layer = create_default_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Home, Modifiers::NONE)]),
            Some(&Command::MoveLineStart)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::End, Modifiers::NONE)]),
            Some(&Command::MoveLineEnd)
        );
    }

    #[test]
    fn default_keymap_ctrl_home_end_buffer() {
        let layer = create_default_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Home, Modifiers::CTRL)]),
            Some(&Command::MoveBufferStart)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::End, Modifiers::CTRL)]),
            Some(&Command::MoveBufferEnd)
        );
    }

    #[test]
    fn default_keymap_name_is_default() {
        let layer = create_default_keymap();
        assert_eq!(layer.name(), "default");
    }

    #[test]
    fn default_keymap_f3_is_find_next() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(Key::F(3), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::FindNext));
    }

    #[test]
    fn default_keymap_shift_f3_is_find_prev() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::new(Key::F(3), Modifiers::SHIFT)];
        assert_eq!(layer.get(&seq), Some(&Command::FindPrev));
    }

    #[test]
    fn default_keymap_ctrl_n_is_find_next() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('n')];
        assert_eq!(layer.get(&seq), Some(&Command::FindNext));
    }

    #[test]
    fn default_keymap_ctrl_o_is_open() {
        let layer = create_default_keymap();
        let seq = vec![KeyEvent::ctrl('o')];
        assert_eq!(layer.get(&seq), Some(&Command::Open));
    }
}
