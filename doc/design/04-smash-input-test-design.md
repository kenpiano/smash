# smash-input — Test Design

## 1. Overview

Test strategy for `smash-input`. Coverage target: ≥ 70%.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-input/tests/` | `cargo test` |

---

## 3. Unit Test Plan

### 3.1 `event.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `event_normalize_ctrl_a` | Raw Ctrl+A event | `KeyEvent { key: Char('a'), modifiers: Ctrl }` |
| `event_normalize_arrow_keys` | Arrow key events | Correct `Key::Arrow*` variants |
| `event_normalize_function_keys` | F1-F12 | Correct `Key::F(n)` |
| `event_normalize_paste` | Bracketed paste sequence | `InputEvent::Paste(text)` |
| `event_resize` | Resize event | `InputEvent::Resize(w, h)` |

### 3.2 `keymap.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `keymap_single_key_binding` | Bind Ctrl-S → Save | Lookup returns `Save` |
| `keymap_multi_key_chord` | Bind Ctrl-K, Ctrl-C → Comment | Lookup returns `Comment` |
| `keymap_override_binding` | Re-bind Ctrl-S → Custom | New binding active |
| `keymap_layer_push_pop` | Push Vim layer, pop it | Bindings switch correctly |
| `keymap_layer_override` | Vim layer overrides default | Vim binding takes priority |

### 3.3 `resolver.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `resolver_single_key_resolves_immediately` | Press Ctrl-S | `ResolveResult::Command(Save)` |
| `resolver_chord_first_key_waits` | Press Ctrl-K (chord prefix) | `ResolveResult::WaitingForMore` |
| `resolver_chord_completes` | Press Ctrl-K then Ctrl-C | `ResolveResult::Command(Comment)` |
| `resolver_chord_timeout_resets` | Press Ctrl-K, wait >1s | State reset, no command |
| `resolver_unbound_key_falls_through` | Press unbound key in normal mode | `ResolveResult::Fallback` |
| `resolver_char_input_in_insert_mode` | Press 'a' with no binding | `ResolveResult::Command(InsertChar('a'))` |
| `resolver_escape_cancels_partial` | Ctrl-K then Esc | State reset |

### 3.4 `command.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `command_id_from_string` | "editor.save" | `CommandId("editor.save")` |
| `command_all_variants_have_ids` | Iterate all Command variants | Each has a unique string ID |

### 3.5 `palette.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `palette_exact_match` | Query "save" | `Save` command ranked first |
| `palette_fuzzy_match` | Query "sv" | `Save` in results |
| `palette_empty_query` | Query "" | All commands returned |
| `palette_no_match` | Query "zzzzz" | Empty results |
| `palette_includes_keybinding_hint` | Query "save" | Result includes "Ctrl-S" |

### 3.6 `default_keymap.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `default_keymap_ctrl_s_saves` | Load default, lookup Ctrl-S | `Save` |
| `default_keymap_ctrl_q_quits` | Load default, lookup Ctrl-Q | `Quit` |
| `default_keymap_ctrl_z_undoes` | Load default, lookup Ctrl-Z | `Undo` |
| `default_keymap_arrows_move` | Load default, lookup arrows | `MoveLeft/Right/Up/Down` |

### 3.7 `vim.rs` (Phase 2)

| Test Name | Scenario | Expected |
|---|---|---|
| `vim_normal_h_moves_left` | 'h' in normal mode | `MoveLeft` |
| `vim_normal_dd_deletes_line` | 'd' then 'd' | `DeleteLine` |
| `vim_i_enters_insert_mode` | 'i' in normal mode | Mode switches to Insert |
| `vim_esc_returns_to_normal` | Esc in insert mode | Mode switches to Normal |
| `vim_v_enters_visual_mode` | 'v' in normal mode | Mode switches to Visual |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `full_keypress_to_command_flow` | Simulate Ctrl-S keypress | `Save` command dispatched |
| `multikey_chord_flow` | Simulate Ctrl-K, Ctrl-C | Command resolved after 2nd key |
| `custom_keymap_from_config` | Load TOML keymap, test bindings | User bindings work |
| `keymap_merge_user_over_default` | User overrides Ctrl-S, rest default | Override active, defaults intact |

---

## 5. Coverage Target

**Minimum: 70%** line coverage.

Priority for near-100%:
- `resolver.rs` — all resolution paths (match, wait, timeout, fallback)
- `keymap.rs` — layer push/pop, override
- `command.rs` — all command variants
