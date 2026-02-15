//! Emacs-style keymap preset for SMASH.
//!
//! Provides a **complete, self-contained** keymap that *replaces* the
//! default keymap entirely.  Unlike a layer-overlay approach, no
//! default bindings can leak through — only the keys explicitly
//! defined here are active.
//!
//! Every binding uses a modifier key (Ctrl or Alt/Meta); there is no
//! modal switching.
//!
//! ## Navigation
//!   Ctrl-f / Ctrl-b — forward / backward character
//!   Ctrl-n / Ctrl-p — next / previous line
//!   Ctrl-a / Ctrl-e — beginning / end of line
//!   Alt-f  / Alt-b  — forward / backward word
//!   Ctrl-v / Alt-v  — page down / page up
//!   Alt-<  / Alt->  — beginning / end of buffer
//!
//! ## Editing
//!   Ctrl-d          — delete forward character
//!   Ctrl-h          — delete backward character
//!   Ctrl-k          — kill (delete) to end of line
//!   Ctrl-/          — undo
//!   Ctrl-z          — undo (convenience alias)
//!
//! ## Search
//!   Ctrl-s          — incremental search (find)
//!   Ctrl-r          — reverse search (find-prev)
//!
//! ## File / Meta
//!   Ctrl-x Ctrl-s   — save
//!   Ctrl-x Ctrl-c   — quit
//!   Ctrl-x Ctrl-f   — open file
//!   Alt-x           — command palette (M-x)
//!   Ctrl-g          — cancel / keyboard-quit (→ Noop)
//!   Ctrl-\\          — toggle terminal

use crate::command::Command;
use crate::event::{Key, KeyEvent, Modifiers};
use crate::keymap::KeymapLayer;

/// Create a complete Emacs-style keymap.
///
/// This returns a **base-layer** keymap — use it as the sole layer in
/// the `Keymap` (via `Keymap::new(create_emacs_keymap())`).
/// No default layer should be pushed underneath it.
pub fn create_emacs_keymap() -> KeymapLayer {
    let mut layer = KeymapLayer::new("emacs");

    // ── Basic editing keys (Enter, Tab, Backspace, Delete) ──────

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

    // ── Emacs navigation (Ctrl) ─────────────────────────────────

    // Ctrl-f — forward character
    layer.bind(vec![KeyEvent::ctrl('f')], Command::MoveRight);
    // Ctrl-b — backward character
    layer.bind(vec![KeyEvent::ctrl('b')], Command::MoveLeft);
    // Ctrl-n — next line
    layer.bind(vec![KeyEvent::ctrl('n')], Command::MoveDown);
    // Ctrl-p — previous line
    layer.bind(vec![KeyEvent::ctrl('p')], Command::MoveUp);
    // Ctrl-a — beginning of line
    layer.bind(vec![KeyEvent::ctrl('a')], Command::MoveLineStart);
    // Ctrl-e — end of line
    layer.bind(vec![KeyEvent::ctrl('e')], Command::MoveLineEnd);

    // ── Emacs navigation (Alt / Meta) ───────────────────────────

    // Alt-f — forward word
    layer.bind(
        vec![KeyEvent::new(Key::Char('f'), Modifiers::ALT)],
        Command::MoveWordRight,
    );
    // Alt-b — backward word
    layer.bind(
        vec![KeyEvent::new(Key::Char('b'), Modifiers::ALT)],
        Command::MoveWordLeft,
    );
    // Ctrl-v — page down
    layer.bind(vec![KeyEvent::ctrl('v')], Command::PageDown);
    // Alt-v — page up
    layer.bind(
        vec![KeyEvent::new(Key::Char('v'), Modifiers::ALT)],
        Command::PageUp,
    );
    // Alt-< — beginning of buffer
    layer.bind(
        vec![KeyEvent::new(Key::Char('<'), Modifiers::ALT)],
        Command::MoveBufferStart,
    );
    // Alt-> — end of buffer
    layer.bind(
        vec![KeyEvent::new(Key::Char('>'), Modifiers::ALT)],
        Command::MoveBufferEnd,
    );

    // ── Editing ─────────────────────────────────────────────────

    // Ctrl-d — delete forward character
    layer.bind(vec![KeyEvent::ctrl('d')], Command::DeleteForward);
    // Ctrl-h — delete backward character (Emacs convention)
    layer.bind(vec![KeyEvent::ctrl('h')], Command::DeleteBackward);
    // Ctrl-k — kill line
    layer.bind(vec![KeyEvent::ctrl('k')], Command::DeleteLine);
    // Ctrl-/ — undo
    layer.bind(vec![KeyEvent::ctrl('/')], Command::Undo);
    // Ctrl-z — undo (convenience alias)
    layer.bind(vec![KeyEvent::ctrl('z')], Command::Undo);

    // ── Search ──────────────────────────────────────────────────

    // Ctrl-s — incremental search (find)
    layer.bind(vec![KeyEvent::ctrl('s')], Command::Find);
    // Ctrl-r — reverse search
    layer.bind(vec![KeyEvent::ctrl('r')], Command::FindPrev);

    // ── File / Meta operations ──────────────────────────────────

    // Alt-x — command palette (M-x)
    layer.bind(
        vec![KeyEvent::new(Key::Char('x'), Modifiers::ALT)],
        Command::OpenCommandPalette,
    );
    // Ctrl-g — cancel / keyboard-quit
    layer.bind(vec![KeyEvent::ctrl('g')], Command::Noop);
    // Ctrl-\ — toggle terminal
    layer.bind(
        vec![KeyEvent::new(Key::Char('\\'), Modifiers::CTRL)],
        Command::ToggleTerminal,
    );

    // ── Two-key chords (Ctrl-x prefix) ─────────────────────────

    // Ctrl-x Ctrl-s — save
    layer.bind(
        vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('s')],
        Command::Save,
    );
    // Ctrl-x Ctrl-c — quit
    layer.bind(
        vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('c')],
        Command::Quit,
    );
    // Ctrl-x Ctrl-f — open file
    layer.bind(
        vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('f')],
        Command::Open,
    );

    // ── Standard (modifier-free) keys ───────────────────────────

    // Arrow keys
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
    // Home / End
    layer.bind(
        vec![KeyEvent::new(Key::Home, Modifiers::NONE)],
        Command::MoveLineStart,
    );
    layer.bind(
        vec![KeyEvent::new(Key::End, Modifiers::NONE)],
        Command::MoveLineEnd,
    );
    // Page Up / Down
    layer.bind(
        vec![KeyEvent::new(Key::PageUp, Modifiers::NONE)],
        Command::PageUp,
    );
    layer.bind(
        vec![KeyEvent::new(Key::PageDown, Modifiers::NONE)],
        Command::PageDown,
    );

    // ── LSP operations ──────────────────────────────────────────

    // Alt-. — go to definition (Emacs style)
    layer.bind(
        vec![KeyEvent::new(Key::Char('.'), Modifiers::ALT)],
        Command::LspGotoDefinition,
    );
    // Alt-? — find references
    layer.bind(
        vec![KeyEvent::new(Key::Char('?'), Modifiers::ALT)],
        Command::LspFindReferences,
    );
    // Ctrl-c Ctrl-r — rename
    layer.bind(
        vec![KeyEvent::ctrl('c'), KeyEvent::ctrl('r')],
        Command::LspRename,
    );
    // Ctrl-c Ctrl-d — hover docs
    layer.bind(
        vec![KeyEvent::ctrl('c'), KeyEvent::ctrl('d')],
        Command::LspHover,
    );
    // Alt-/ — completion
    layer.bind(
        vec![KeyEvent::new(Key::Char('/'), Modifiers::ALT)],
        Command::LspCompletion,
    );
    // Ctrl-c Ctrl-f — format
    layer.bind(
        vec![KeyEvent::ctrl('c'), KeyEvent::ctrl('f')],
        Command::LspFormat,
    );
    // Ctrl-c Ctrl-a — code action
    layer.bind(
        vec![KeyEvent::ctrl('c'), KeyEvent::ctrl('a')],
        Command::LspCodeAction,
    );
    // Alt-n / Alt-p — next/prev diagnostic
    layer.bind(
        vec![KeyEvent::new(Key::Char('n'), Modifiers::ALT)],
        Command::LspDiagnosticNext,
    );
    layer.bind(
        vec![KeyEvent::new(Key::Char('p'), Modifiers::ALT)],
        Command::LspDiagnosticPrev,
    );
    // F12 — go to definition (also standard)
    layer.bind(
        vec![KeyEvent::new(Key::F(12), Modifiers::NONE)],
        Command::LspGotoDefinition,
    );

    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Layer identity ──────────────────────────────────────────

    #[test]
    fn emacs_keymap_name_is_emacs() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.name(), "emacs");
    }

    // ── Basic editing keys ──────────────────────────────────────

    #[test]
    fn emacs_enter_inserts_newline() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Enter, Modifiers::NONE)]),
            Some(&Command::InsertNewline)
        );
    }

    #[test]
    fn emacs_backspace_deletes_backward() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Backspace, Modifiers::NONE)]),
            Some(&Command::DeleteBackward)
        );
    }

    #[test]
    fn emacs_tab_inserts_tab() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Tab, Modifiers::NONE)]),
            Some(&Command::InsertChar('\t'))
        );
    }

    #[test]
    fn emacs_delete_deletes_forward() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Delete, Modifiers::NONE)]),
            Some(&Command::DeleteForward)
        );
    }

    // ── Navigation: single-key Ctrl bindings ────────────────────

    #[test]
    fn emacs_ctrl_f_moves_right() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('f')]), Some(&Command::MoveRight));
    }

    #[test]
    fn emacs_ctrl_b_moves_left() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('b')]), Some(&Command::MoveLeft));
    }

    #[test]
    fn emacs_ctrl_n_moves_down() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('n')]), Some(&Command::MoveDown));
    }

    #[test]
    fn emacs_ctrl_p_moves_up() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('p')]), Some(&Command::MoveUp));
    }

    #[test]
    fn emacs_ctrl_a_moves_line_start() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::ctrl('a')]),
            Some(&Command::MoveLineStart)
        );
    }

    #[test]
    fn emacs_ctrl_e_moves_line_end() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::ctrl('e')]),
            Some(&Command::MoveLineEnd)
        );
    }

    // ── Navigation: Alt (Meta) bindings ─────────────────────────

    #[test]
    fn emacs_alt_f_moves_word_right() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('f'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveWordRight));
    }

    #[test]
    fn emacs_alt_b_moves_word_left() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('b'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveWordLeft));
    }

    #[test]
    fn emacs_ctrl_v_pages_down() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('v')]), Some(&Command::PageDown));
    }

    #[test]
    fn emacs_alt_v_pages_up() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('v'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::PageUp));
    }

    #[test]
    fn emacs_alt_less_moves_buffer_start() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('<'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveBufferStart));
    }

    #[test]
    fn emacs_alt_greater_moves_buffer_end() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('>'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::MoveBufferEnd));
    }

    // ── Editing ─────────────────────────────────────────────────

    #[test]
    fn emacs_ctrl_d_deletes_forward() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::ctrl('d')]),
            Some(&Command::DeleteForward)
        );
    }

    #[test]
    fn emacs_ctrl_h_deletes_backward() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::ctrl('h')]),
            Some(&Command::DeleteBackward)
        );
    }

    #[test]
    fn emacs_ctrl_k_deletes_line() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::ctrl('k')]),
            Some(&Command::DeleteLine)
        );
    }

    #[test]
    fn emacs_ctrl_slash_undoes() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('/')]), Some(&Command::Undo));
    }

    #[test]
    fn emacs_ctrl_z_undoes() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('z')]), Some(&Command::Undo));
    }

    // ── Search ──────────────────────────────────────────────────

    #[test]
    fn emacs_ctrl_s_finds() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('s')]), Some(&Command::Find));
    }

    #[test]
    fn emacs_ctrl_r_finds_prev() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('r')]), Some(&Command::FindPrev));
    }

    // ── File / Meta ─────────────────────────────────────────────

    #[test]
    fn emacs_alt_x_opens_command_palette() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('x'), Modifiers::ALT)];
        assert_eq!(layer.get(&seq), Some(&Command::OpenCommandPalette));
    }

    #[test]
    fn emacs_ctrl_g_is_noop() {
        let layer = create_emacs_keymap();
        assert_eq!(layer.get(&[KeyEvent::ctrl('g')]), Some(&Command::Noop));
    }

    #[test]
    fn emacs_ctrl_backslash_toggle_terminal() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::new(Key::Char('\\'), Modifiers::CTRL)];
        assert_eq!(layer.get(&seq), Some(&Command::ToggleTerminal));
    }

    // ── Two-key file operations ─────────────────────────────────

    #[test]
    fn emacs_ctrl_x_ctrl_s_saves() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('s')];
        assert_eq!(layer.get(&seq), Some(&Command::Save));
    }

    #[test]
    fn emacs_ctrl_x_ctrl_c_quits() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('c')];
        assert_eq!(layer.get(&seq), Some(&Command::Quit));
    }

    #[test]
    fn emacs_ctrl_x_ctrl_f_opens() {
        let layer = create_emacs_keymap();
        let seq = vec![KeyEvent::ctrl('x'), KeyEvent::ctrl('f')];
        assert_eq!(layer.get(&seq), Some(&Command::Open));
    }

    // ── Prefix detection ────────────────────────────────────────

    #[test]
    fn emacs_ctrl_x_is_prefix() {
        let layer = create_emacs_keymap();
        let prefix = vec![KeyEvent::ctrl('x')];
        assert!(layer.has_prefix(&prefix));
    }

    // ── Standard keys ───────────────────────────────────────────

    #[test]
    fn emacs_arrow_keys_work() {
        let layer = create_emacs_keymap();
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
    fn emacs_home_end_keys_work() {
        let layer = create_emacs_keymap();
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
    fn emacs_page_keys_work() {
        let layer = create_emacs_keymap();
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageUp, Modifiers::NONE)]),
            Some(&Command::PageUp)
        );
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::PageDown, Modifiers::NONE)]),
            Some(&Command::PageDown)
        );
    }

    // ── No leaking: default bindings must NOT be present ────────

    #[test]
    fn emacs_no_ctrl_w_close_pane() {
        let layer = create_emacs_keymap();
        // Ctrl-w must NOT map to ClosePane (default leak)
        assert_eq!(layer.get(&[KeyEvent::ctrl('w')]), None);
    }

    #[test]
    fn emacs_no_ctrl_o_open() {
        let layer = create_emacs_keymap();
        // Ctrl-o must NOT map to Open (default leak); unbound
        assert_eq!(layer.get(&[KeyEvent::ctrl('o')]), None);
    }

    #[test]
    fn emacs_no_ctrl_q_quit() {
        let layer = create_emacs_keymap();
        // Ctrl-q must NOT quit — emacs uses Ctrl-x Ctrl-c
        assert_eq!(layer.get(&[KeyEvent::ctrl('q')]), None);
    }

    #[test]
    fn emacs_unbound_char_returns_none() {
        let layer = create_emacs_keymap();
        // Plain 'a' should not be bound — resolver handles InsertChar
        assert_eq!(
            layer.get(&[KeyEvent::new(Key::Char('a'), Modifiers::NONE)]),
            None
        );
    }
}
