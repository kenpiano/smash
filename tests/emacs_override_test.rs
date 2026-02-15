use smash_input::{
    create_emacs_keymap, Command, InputEvent, Key, KeyEvent, KeyResolver, Keymap, Modifiers,
    ResolveResult,
};

// ── Standalone emacs keymap tests (no default layer) ────────────

fn emacs_resolver() -> KeyResolver {
    let keymap = Keymap::new(create_emacs_keymap());
    KeyResolver::new(keymap)
}

#[test]
fn emacs_only_layer_count_is_one() {
    let keymap = Keymap::new(create_emacs_keymap());
    assert_eq!(keymap.layer_count(), 1, "emacs must be sole layer");
}

#[test]
fn emacs_no_default_leak_ctrl_w() {
    let keymap = Keymap::new(create_emacs_keymap());
    assert_eq!(
        keymap.resolve(&[KeyEvent::ctrl('w')]),
        None,
        "Ctrl-w must NOT leak ClosePane from absent default layer"
    );
}

#[test]
fn emacs_no_default_leak_ctrl_o() {
    let keymap = Keymap::new(create_emacs_keymap());
    assert_eq!(
        keymap.resolve(&[KeyEvent::ctrl('o')]),
        None,
        "Ctrl-o must NOT leak Open from absent default layer"
    );
}

#[test]
fn emacs_no_default_leak_ctrl_q() {
    let keymap = Keymap::new(create_emacs_keymap());
    assert_eq!(
        keymap.resolve(&[KeyEvent::ctrl('q')]),
        None,
        "Ctrl-q must NOT leak Quit from absent default layer"
    );
}

#[test]
fn resolver_emacs_ctrl_f_produces_move_right() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('f')));
    assert_eq!(result, ResolveResult::Command(Command::MoveRight));
}

#[test]
fn resolver_emacs_ctrl_n_produces_move_down() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('n')));
    assert_eq!(result, ResolveResult::Command(Command::MoveDown));
}

#[test]
fn resolver_emacs_ctrl_p_produces_move_up() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('p')));
    assert_eq!(result, ResolveResult::Command(Command::MoveUp));
}

#[test]
fn resolver_emacs_ctrl_a_produces_line_start() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('a')));
    assert_eq!(result, ResolveResult::Command(Command::MoveLineStart));
}

#[test]
fn resolver_emacs_ctrl_s_produces_find() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('s')));
    assert_eq!(result, ResolveResult::Command(Command::Find));
}

#[test]
fn resolver_emacs_ctrl_x_waits_for_chord() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('x')));
    assert_eq!(result, ResolveResult::WaitingForMore);
}

#[test]
fn resolver_emacs_ctrl_x_ctrl_s_saves() {
    let mut r = emacs_resolver();
    let r1 = r.resolve(InputEvent::Key(KeyEvent::ctrl('x')));
    assert_eq!(r1, ResolveResult::WaitingForMore);
    let r2 = r.resolve(InputEvent::Key(KeyEvent::ctrl('s')));
    assert_eq!(r2, ResolveResult::Command(Command::Save));
}

#[test]
fn resolver_emacs_ctrl_x_ctrl_c_quits() {
    let mut r = emacs_resolver();
    let r1 = r.resolve(InputEvent::Key(KeyEvent::ctrl('x')));
    assert_eq!(r1, ResolveResult::WaitingForMore);
    let r2 = r.resolve(InputEvent::Key(KeyEvent::ctrl('c')));
    assert_eq!(r2, ResolveResult::Command(Command::Quit));
}

#[test]
fn resolver_emacs_alt_f_word_right() {
    let mut r = emacs_resolver();
    let ke = KeyEvent::new(Key::Char('f'), Modifiers::ALT);
    let result = r.resolve(InputEvent::Key(ke));
    assert_eq!(result, ResolveResult::Command(Command::MoveWordRight));
}

#[test]
fn resolver_emacs_alt_b_word_left() {
    let mut r = emacs_resolver();
    let ke = KeyEvent::new(Key::Char('b'), Modifiers::ALT);
    let result = r.resolve(InputEvent::Key(ke));
    assert_eq!(result, ResolveResult::Command(Command::MoveWordLeft));
}

#[test]
fn resolver_emacs_plain_char_still_inserts() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::char('a')));
    assert_eq!(result, ResolveResult::Command(Command::InsertChar('a')));
}

#[test]
fn resolver_emacs_ctrl_z_directly_undoes() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('z')));
    assert_eq!(result, ResolveResult::Command(Command::Undo));
}

#[test]
fn resolver_emacs_enter_inserts_newline() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::new(Key::Enter, Modifiers::NONE)));
    assert_eq!(result, ResolveResult::Command(Command::InsertNewline));
}

#[test]
fn resolver_emacs_backspace_deletes_backward() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::new(
        Key::Backspace,
        Modifiers::NONE,
    )));
    assert_eq!(result, ResolveResult::Command(Command::DeleteBackward));
}

#[test]
fn resolver_emacs_ctrl_g_is_noop() {
    let mut r = emacs_resolver();
    let result = r.resolve(InputEvent::Key(KeyEvent::ctrl('g')));
    assert_eq!(result, ResolveResult::Command(Command::Noop));
}
