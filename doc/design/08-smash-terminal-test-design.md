# smash-terminal — Test Design

## 1. Overview

Test strategy for `smash-terminal`. Requires ≥ 80% coverage (core crate).

Terminal testing uses a **mock PTY** that provides controlled input/output without spawning a real shell.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-terminal/tests/` | `cargo test` |
| Fuzz tests | `fuzz/` | `cargo-fuzz` (VT parser) |

---

## 3. Unit Test Plan

### 3.1 `grid.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `grid_new_has_correct_dimensions` | 80×24 grid | All cells empty/default |
| `grid_set_cell_content` | Write 'A' at (0,0) | Cell contains 'A' |
| `grid_cursor_move_right` | Move cursor right | Column incremented |
| `grid_cursor_wraps_at_eol` | Cursor at last column, write char | Moves to next line |
| `grid_scroll_up` | Scroll region, add line at bottom | Top line lost, new blank at bottom |
| `grid_scroll_down` | Scroll down | Bottom line lost, new blank at top |
| `grid_alternate_screen_switch` | Enter alternate screen → content → leave | Primary screen restored |
| `grid_erase_display` | Clear screen | All cells empty |
| `grid_erase_line` | Clear current line | Line cells empty |
| `grid_scrollback_preserved` | Scroll past top | Lines in scrollback buffer |

### 3.2 `parser.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `parser_plain_text` | Feed "hello" | Grid shows "hello" |
| `parser_csi_cursor_up` | Feed `\x1b[A` | Cursor moves up 1 |
| `parser_csi_cursor_position` | Feed `\x1b[5;10H` | Cursor at (4,9) |
| `parser_sgr_bold` | Feed `\x1b[1m` | Subsequent text is bold |
| `parser_sgr_fg_color` | Feed `\x1b[38;5;196m` | FG color set to 196 |
| `parser_sgr_rgb_color` | Feed `\x1b[38;2;255;0;0m` | FG color set to red |
| `parser_sgr_reset` | Feed `\x1b[0m` | Attributes reset to default |
| `parser_osc_set_title` | Feed `\x1b]0;My Title\x07` | Title event emitted |
| `parser_scroll_region` | Feed `\x1b[5;20r` | Scroll region set |
| `parser_alternate_screen` | Feed `\x1b[?1049h` then `\x1b[?1049l` | Switch and restore |

### 3.3 `input.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `input_plain_char` | 'a' | Byte `0x61` |
| `input_enter` | Enter key | `\r` |
| `input_arrow_up` | Arrow Up | `\x1b[A` |
| `input_ctrl_c` | Ctrl-C | `\x03` |
| `input_function_key_f1` | F1 | `\x1bOP` |

### 3.4 `hyperlink.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `hyperlink_detect_url` | Grid contains "https://example.com" | Hyperlink annotation |
| `hyperlink_detect_file_path` | Grid contains "/home/user/file.rs:10" | Path hyperlink |
| `hyperlink_no_false_positive` | Grid contains "not a link" | No annotations |
| `hyperlink_osc8_explicit` | OSC 8 hyperlink sequence | Explicit hyperlink set |

### 3.5 `pty.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `pty_create_and_spawn` | Create PTY, spawn `echo hello` | Output contains "hello" |
| `pty_resize` | Resize PTY to 100×50 | No error, SIGWINCH sent |
| `pty_write_and_read` | Write to stdin, read stdout | Data round-trips |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `terminal_spawn_shell` | Spawn default shell | Grid shows prompt |
| `terminal_echo_command` | Type `echo hello\n` | Grid shows "hello" in output |
| `terminal_resize_updates_grid` | Resize from 80×24 to 120×40 | Grid dimensions match |
| `terminal_close_terminates` | Close terminal | PTY process exits |
| `terminal_multiple_instances` | Spawn 3 terminals | All independent |

---

## 5. Fuzz Testing

The VT parser is fuzz-tested to ensure robustness against arbitrary byte sequences:

```rust
// fuzz/fuzz_targets/vt_parser.rs
fuzz_target!(|data: &[u8]| {
    let mut grid = TerminalGrid::new(80, 24);
    let mut parser = VtParser::new();
    parser.process(data, &mut grid);
    // No panics, no UB
});
```

---

## 6. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_parse_1kb_plain_text` | < 100 µs |
| `bench_parse_1kb_heavy_escapes` | < 500 µs |
| `bench_grid_scroll_1000_lines` | < 1 ms |
| `bench_hyperlink_scan_80x24` | < 500 µs |

---

## 7. Coverage Target

**Minimum: 80%** line coverage.

Priority for near-100%:
- `parser.rs` — all CSI/SGR/OSC code paths
- `grid.rs` — scroll, erase, cursor movement edge cases
- `pty.rs` — error paths
