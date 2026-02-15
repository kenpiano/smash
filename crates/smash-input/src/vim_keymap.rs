//! Vim-style keymap preset for SMASH.
//!
//! Provides a Vim Normal mode keymap layer. When active, single-key
//! commands (h/j/k/l for movement, i/a for insert, etc.) take
//! precedence over character insertion.
//!
//! Usage: push `create_vim_normal_layer()` onto the keymap stack to
//! enter Vim Normal mode. Pop it to return to insert mode.
use crate::command::Command;
use crate::event::{Key, KeyEvent, Modifiers};
use crate::keymap::KeymapLayer;

/// Create a Vim Normal mode keymap layer.
///
/// Binds Vim-style single-key commands for navigation, editing, and
/// file operations. This layer should be pushed on top of the default
/// layer. When the user enters insert mode (i/a/o), this layer should
/// be popped.
pub fn create_vim_normal_layer() -> KeymapLayer {
    let mut layer = KeymapLayer::new("vim-normal");

    // --- Movement ---
    // h/j/k/l
    layer.bind(
        vec![KeyEvent::new(Key::Char('h'), Modifiers::NONE)],
        Command::MoveLeft,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('j'), Modifiers::NONE)],
        Command::MoveDown,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('k'), Modifiers::NONE)],
        Command::MoveUp,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('l'), Modifiers::NONE)],
        Command::MoveRight,
    );

    // w/b — word movement
    layer.bind(
        vec![KeyEvent::new(Key::Char('w'), Modifiers::NONE)],
        Command::MoveWordRight,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('b'), Modifiers::NONE)],
        Command::MoveWordLeft,
    );

    // 0 — line start, $ — line end
    layer.bind(
        vec![KeyEvent::new(Key::Char('0'), Modifiers::NONE)],
        Command::MoveLineStart,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('$'), Modifiers::NONE)],
        Command::MoveLineEnd,
    );

    // gg — buffer start (two key sequence)
    layer.bind(
        vec![
            KeyEvent::new(Key::Char('g'), Modifiers::NONE),
            KeyEvent::new(Key::Char('g'), Modifiers::NONE),
        ],
        Command::MoveBufferStart,
    );

    // G — buffer end
    layer.bind(
        vec![KeyEvent::new(Key::Char('G'), Modifiers::SHIFT)],
        Command::MoveBufferEnd,
    );

    // --- Editing ---
    // x — delete character at cursor
    layer.bind(
        vec![KeyEvent::new(Key::Char('x'), Modifiers::NONE)],
        Command::DeleteForward,
    );

    // dd — delete line
    layer.bind(
        vec![
            KeyEvent::new(Key::Char('d'), Modifiers::NONE),
            KeyEvent::new(Key::Char('d'), Modifiers::NONE),
        ],
        Command::DeleteLine,
    );

    // u — undo
    layer.bind(
        vec![KeyEvent::new(Key::Char('u'), Modifiers::NONE)],
        Command::Undo,
    );

    // Ctrl-R — redo
    layer.bind(vec![KeyEvent::ctrl('r')], Command::Redo);

    // --- Search ---
    // / — find
    layer.bind(
        vec![KeyEvent::new(Key::Char('/'), Modifiers::NONE)],
        Command::Find,
    );
    // n — next match
    layer.bind(
        vec![KeyEvent::new(Key::Char('n'), Modifiers::NONE)],
        Command::FindNext,
    );
    // N — prev match
    layer.bind(
        vec![KeyEvent::new(Key::Char('N'), Modifiers::SHIFT)],
        Command::FindPrev,
    );

    // --- File operations ---
    // :w shortcut — Ctrl-S still works from base layer
    layer.bind(
        vec![KeyEvent::new(Key::Char(':'), Modifiers::NONE)],
        Command::OpenCommandPalette,
    );

    // --- Mode switching ---
    // i — enter insert mode (Noop → handled by main.rs to pop layer)
    layer.bind(
        vec![KeyEvent::new(Key::Char('i'), Modifiers::NONE)],
        Command::Noop,
    );
    // a — enter insert mode after cursor
    layer.bind(
        vec![KeyEvent::new(Key::Char('a'), Modifiers::NONE)],
        Command::Noop,
    );
    // o — open line below and enter insert
    layer.bind(
        vec![KeyEvent::new(Key::Char('o'), Modifiers::NONE)],
        Command::Noop,
    );

    // --- Panes ---
    // Ctrl-W + v — split vertical
    layer.bind(
        vec![
            KeyEvent::ctrl('w'),
            KeyEvent::new(Key::Char('v'), Modifiers::NONE),
        ],
        Command::SplitVertical,
    );
    // Ctrl-W + s — split horizontal
    layer.bind(
        vec![
            KeyEvent::ctrl('w'),
            KeyEvent::new(Key::Char('s'), Modifiers::NONE),
        ],
        Command::SplitHorizontal,
    );

    // Page Up/Down — keep standard bindings
    layer.bind(
        vec![KeyEvent::new(Key::PageUp, Modifiers::NONE)],
        Command::PageUp,
    );
    layer.bind(
        vec![KeyEvent::new(Key::PageDown, Modifiers::NONE)],
        Command::PageDown,
    );

    // Arrow keys still work
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

    layer
}

/// Create a Vim Insert mode keymap layer.
///
/// This is essentially transparent: most keys fall through to
/// the default layer for character insertion. Only Esc is bound to
/// return to Normal mode.
pub fn create_vim_insert_layer() -> KeymapLayer {
    let mut layer = KeymapLayer::new("vim-insert");

    // Esc → returns Noop, which main.rs interprets as "switch to Normal mode"
    layer.bind(
        vec![KeyEvent::new(Key::Esc, Modifiers::NONE)],
        Command::Noop,
    );

    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_normal_name() {
        let layer = create_vim_normal_layer();
        assert_eq!(layer.name(), "vim-normal");
    }

    #[test]
    fn vim_insert_name() {
        let layer = create_vim_insert_layer();
        assert_eq!(layer.name(), "vim-insert");
    }

    // --- Movement tests ---

    #[test]
    fn vim_h_moves_left() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('h'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveLeft));
    }

    #[test]
    fn vim_j_moves_down() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('j'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveDown));
    }

    #[test]
    fn vim_k_moves_up() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('k'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveUp));
    }

    #[test]
    fn vim_l_moves_right() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('l'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveRight));
    }

    #[test]
    fn vim_w_moves_word_right() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('w'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveWordRight));
    }

    #[test]
    fn vim_b_moves_word_left() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('b'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveWordLeft));
    }

    #[test]
    fn vim_0_moves_line_start() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('0'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveLineStart));
    }

    #[test]
    fn vim_dollar_moves_line_end() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('$'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveLineEnd));
    }

    #[test]
    fn vim_gg_moves_buffer_start() {
        let layer = create_vim_normal_layer();
        let seq = vec![
            KeyEvent::new(Key::Char('g'), Modifiers::NONE),
            KeyEvent::new(Key::Char('g'), Modifiers::NONE),
        ];
        assert_eq!(layer.get(&seq), Some(&Command::MoveBufferStart));
    }

    #[test]
    fn vim_shift_g_moves_buffer_end() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('G'), Modifiers::SHIFT)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveBufferEnd));
    }

    // --- Editing tests ---

    #[test]
    fn vim_x_deletes_forward() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('x'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::DeleteForward));
    }

    #[test]
    fn vim_dd_deletes_line() {
        let layer = create_vim_normal_layer();
        let seq = vec![
            KeyEvent::new(Key::Char('d'), Modifiers::NONE),
            KeyEvent::new(Key::Char('d'), Modifiers::NONE),
        ];
        assert_eq!(layer.get(&seq), Some(&Command::DeleteLine));
    }

    #[test]
    fn vim_u_undoes() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('u'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Undo));
    }

    #[test]
    fn vim_ctrl_r_redoes() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::ctrl('r')];
        assert_eq!(layer.get(&seq), Some(&Command::Redo));
    }

    // --- Search tests ---

    #[test]
    fn vim_slash_finds() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('/'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Find));
    }

    #[test]
    fn vim_n_find_next() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('n'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::FindNext));
    }

    #[test]
    fn vim_shift_n_find_prev() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('N'), Modifiers::SHIFT)];
        assert_eq!(layer.get(&seq), Some(&Command::FindPrev));
    }

    // --- Mode switching tests ---

    #[test]
    fn vim_i_returns_noop() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('i'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Noop));
    }

    #[test]
    fn vim_a_returns_noop() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('a'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Noop));
    }

    #[test]
    fn vim_o_returns_noop() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char('o'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Noop));
    }

    // --- Panes ---

    #[test]
    fn vim_ctrl_w_v_splits_vertical() {
        let layer = create_vim_normal_layer();
        let seq = vec![
            KeyEvent::ctrl('w'),
            KeyEvent::new(Key::Char('v'), Modifiers::NONE),
        ];
        assert_eq!(layer.get(&seq), Some(&Command::SplitVertical));
    }

    #[test]
    fn vim_ctrl_w_s_splits_horizontal() {
        let layer = create_vim_normal_layer();
        let seq = vec![
            KeyEvent::ctrl('w'),
            KeyEvent::new(Key::Char('s'), Modifiers::NONE),
        ];
        assert_eq!(layer.get(&seq), Some(&Command::SplitHorizontal));
    }

    // --- Insert mode tests ---

    #[test]
    fn vim_insert_esc_returns_noop() {
        let layer = create_vim_insert_layer();
        let seq = vec![KeyEvent::new(Key::Esc, Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::Noop));
    }

    #[test]
    fn vim_insert_no_other_bindings() {
        let layer = create_vim_insert_layer();
        // Regular chars should not be bound
        let seq = vec![KeyEvent::new(Key::Char('a'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), None);
    }

    // --- Prefix tests ---

    #[test]
    fn vim_g_is_prefix() {
        let layer = create_vim_normal_layer();
        let prefix = vec![KeyEvent::new(Key::Char('g'), Modifiers::NONE)];
        assert!(layer.has_prefix(&prefix));
    }

    #[test]
    fn vim_d_is_prefix() {
        let layer = create_vim_normal_layer();
        let prefix = vec![KeyEvent::new(Key::Char('d'), Modifiers::NONE)];
        assert!(layer.has_prefix(&prefix));
    }

    #[test]
    fn vim_ctrl_w_is_prefix() {
        let layer = create_vim_normal_layer();
        let prefix = vec![KeyEvent::ctrl('w')];
        assert!(layer.has_prefix(&prefix));
    }

    // --- Colon command ---

    #[test]
    fn vim_colon_opens_command_palette() {
        let layer = create_vim_normal_layer();
        let seq = vec![KeyEvent::new(Key::Char(':'), Modifiers::NONE)];
        assert_eq!(layer.get(&seq), Some(&Command::OpenCommandPalette));
    }

    // --- Arrow key compatibility ---

    #[test]
    fn vim_arrow_keys_work() {
        let layer = create_vim_normal_layer();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Left, Modifiers::NONE)]),
            Some(&Command::MoveLeft)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Right, Modifiers::NONE)]),
            Some(&Command::MoveRight)
        );
    }

    #[test]
    fn vim_page_up_down() {
        let layer = create_vim_normal_layer();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageUp, Modifiers::NONE)]),
            Some(&Command::PageUp)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageDown, Modifiers::NONE)]),
            Some(&Command::PageDown)
        );
    }
}
