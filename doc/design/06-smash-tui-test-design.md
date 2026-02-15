# smash-tui — Test Design

## 1. Overview

Test strategy for `smash-tui`. Coverage target: ≥ 70%.

The renderer is tested primarily through a **mock terminal backend** that captures all output, allowing assertions on screen content without a real terminal.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-tui/tests/` | `cargo test` |
| Benchmarks | `crates/smash-tui/benches/` | `criterion` |

---

## 3. Unit Test Plan

### 3.1 `screen.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `screen_new_has_correct_dimensions` | Create 80×24 screen | `width=80, height=24` |
| `screen_set_cell_and_read` | Set cell at (5,10) | Correct char and style |
| `screen_clear_resets_all_cells` | Clear after writes | All cells = default |
| `screen_diff_detects_single_change` | Change one cell | Diff contains 1 entry |
| `screen_diff_identical_is_empty` | Two identical screens | Empty diff |
| `screen_diff_all_changed` | Completely different screens | All cells in diff |
| `screen_resize_preserves_content` | Resize larger | Existing content intact |

### 3.2 `pane.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `pane_single_fills_screen` | One pane, 80×24 | Pane rect = full screen |
| `pane_split_vertical_halves` | Split vertically | Two panes, each ~40 wide |
| `pane_split_horizontal_halves` | Split horizontally | Two panes, each ~12 tall |
| `pane_close_restores_sibling` | Close one of two panes | Remaining fills screen |
| `pane_focus_next_cycles` | 3 panes, focus next 3x | Returns to first |
| `pane_resize_divider` | Drag divider | Pane proportions updated |
| `pane_nested_splits` | Split A vertically, then split left horizontally | 3 panes, correct rects |

### 3.3 `theme.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `theme_load_builtin_dark` | Load "dark" theme | Valid Theme object |
| `theme_load_builtin_light` | Load "light" theme | Valid Theme object |
| `theme_scope_to_style` | Look up "keyword" | Returns correct CellStyle |
| `theme_unknown_scope_fallback` | Look up "nonexistent" | Falls back to default style |
| `theme_invalid_toml_returns_error` | Malformed TOML | `RenderError::ThemeParse` |
| `theme_not_found_returns_error` | Load "missing" | `RenderError::ThemeNotFound` |

### 3.4 `viewport.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `viewport_scroll_down` | Scroll down 5 lines | `top_line` increases by 5 |
| `viewport_scroll_clamps_to_end` | Scroll past last line | Clamped to max |
| `viewport_scroll_up_clamps_to_zero` | Scroll up past line 0 | `top_line = 0` |
| `viewport_follow_cursor` | Cursor moves below viewport | Viewport scrolls to show cursor |
| `viewport_horizontal_scroll` | Long line, cursor moves right | `left_col` adjusts |

### 3.5 `line_render.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `line_render_plain_text` | No highlights | All default style |
| `line_render_with_highlight_spans` | One keyword span | Cells in range have keyword style |
| `line_render_overlapping_spans` | Syntax + search highlight | Later span (search) takes priority |
| `line_render_cursor_visible` | Cursor at col 5 | Cell at col 5 has cursor style |
| `line_render_selection_background` | Selection range | Cells have selection bg color |
| `line_render_tab_expansion` | Tab character | Expanded to spaces (configurable width) |

### 3.6 `status_bar.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `status_bar_shows_filename` | Buffer with path "foo.rs" | "foo.rs" visible |
| `status_bar_shows_cursor_position` | Cursor at (10, 5) | "11:6" displayed (1-based) |
| `status_bar_shows_dirty_indicator` | Buffer is dirty | "[+]" or "modified" shown |
| `status_bar_shows_encoding` | UTF-8 buffer | "UTF-8" displayed |

### 3.7 `gutter.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `gutter_line_numbers_correct` | 100-line file | Numbers 1-100 displayed |
| `gutter_width_adjusts` | 10000-line file | Gutter is 5 chars wide |
| `gutter_relative_numbers` | Relative mode, cursor at line 50 | Shows 0 at 50, relative above/below |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `render_empty_buffer` | Render fresh empty editor | Screen non-empty (gutter, status bar), no crash |
| `render_with_content` | Render buffer with 50 lines | Lines displayed, syntax highlighted |
| `render_after_edit` | Insert text, re-render | Only changed cells in diff |
| `render_split_panes` | Two panes side by side | Both visible with divider |
| `render_theme_switch` | Switch from dark to light | All cells re-styled |

---

## 5. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_render_full_screen_80x24` | < 2 ms |
| `bench_render_full_screen_200x50` | < 8 ms |
| `bench_screen_diff_80x24` | < 500 µs |
| `bench_screen_diff_200x50` | < 2 ms |
| `bench_line_render_with_spans` | < 50 µs per line |
| `bench_theme_load` | < 5 ms |

---

## 6. Mock Backend

```rust
/// Test backend that captures written bytes
struct MockBackend {
    output: Vec<u8>,
    size: (u16, u16),
}
```

All integration tests use `MockBackend` to verify rendered output without requiring a real terminal.

---

## 7. Coverage Target

**Minimum: 70%** line coverage.

Priority for near-100%:
- `screen.rs` — diff logic
- `pane.rs` — split/close/resize edge cases
- `viewport.rs` — scroll boundary conditions
