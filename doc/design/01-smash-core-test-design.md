# smash-core — Test Design

## 1. Overview

Test strategy for `smash-core`, the most critical crate in SMASH. Requires ≥ 80% coverage (REQ-NFR-030).

**Testing approach**: Unit tests for each module + property-based tests for the rope and undo tree + integration tests for cross-module edit flows.

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test -p smash-core` |
| Property tests | `src/*.rs` (inline) | `proptest` |
| Integration tests | `crates/smash-core/tests/` | `cargo test -p smash-core` |
| Benchmarks | `crates/smash-core/benches/` | `criterion` |

---

## 3. Unit Test Plan

### 3.1 `buffer.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `buffer_new_empty_has_zero_length` | Create empty buffer | `len() == 0`, not dirty |
| `buffer_from_string_preserves_content` | Create from "hello\nworld" | Correct line count and content |
| `buffer_dirty_flag_set_after_edit` | Insert text | `is_dirty() == true` |
| `buffer_dirty_flag_cleared_after_save` | Edit then save | `is_dirty() == false` |
| `buffer_line_count_accurate` | Various inputs | Correct line counts |
| `buffer_line_at_returns_correct_line` | Multi-line buffer | Each line matches |

### 3.2 `rope.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `rope_insert_at_beginning` | Insert at offset 0 | Text prepended |
| `rope_insert_at_end` | Insert at len | Text appended |
| `rope_insert_at_middle` | Insert at midpoint | Text spliced correctly |
| `rope_delete_range` | Delete a byte range | Content removed, length reduced |
| `rope_delete_empty_range` | Delete 0-length range | No change |
| `rope_line_to_byte_offset` | Various line numbers | Correct byte offsets |
| `rope_byte_to_line_conversion` | Various byte offsets | Correct line numbers |
| `rope_large_file_operations` | 100 MB synthetic file | Operations complete within bounds |

**Property tests** (`proptest`):
- Random insert + delete sequences on a rope produce the same result as the same operations on a plain `String`.
- Line index always consistent with byte index after arbitrary edits.

### 3.3 `edit.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `edit_insert_single_char` | Insert 'a' at position (0,0) | Buffer contains "a" |
| `edit_insert_multiline` | Insert "foo\nbar" | Line count increases by 1 |
| `edit_delete_single_char` | Delete one char | Buffer shortened by 1 |
| `edit_delete_across_lines` | Delete range spanning 2 lines | Lines merged |
| `edit_replace_range` | Replace "old" with "new" | Content updated |
| `edit_batch_applies_atomically` | Batch of 3 edits | All applied; single undo step |
| `edit_out_of_bounds_returns_error` | Insert beyond buffer end | `EditError::OutOfBounds` |
| `edit_invalid_range_returns_error` | Delete with start > end | `EditError::InvalidRange` |

### 3.4 `undo.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `undo_reverts_last_edit` | Insert then undo | Buffer empty again |
| `undo_redo_restores_edit` | Insert, undo, redo | Buffer has text again |
| `undo_multiple_steps` | 3 inserts, undo 2x | Only first insert remains |
| `undo_tree_branch_on_edit_after_undo` | Insert, undo, insert different | Two branches in tree |
| `undo_tree_navigate_branches` | Create 2 branches, switch | Correct content per branch |
| `undo_group_reverts_as_one` | Grouped 3 edits, undo once | All 3 reverted |
| `undo_empty_tree_undo_is_noop` | Undo on fresh buffer | No change, no error |
| `undo_cursor_restored_on_undo` | Edit moves cursor, undo | Cursor at original position |

### 3.5 `cursor.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `cursor_move_right_advances` | Move right 1 | Column incremented |
| `cursor_move_right_at_eol_wraps` | Move right at end of line | Next line, column 0 |
| `cursor_move_left_at_bol_wraps` | Move left at start of line | Previous line, end |
| `cursor_move_down_preserves_column` | Move down | Same column, next line |
| `cursor_move_down_clamps_short_line` | Move down to shorter line | Column clamped |
| `cursor_move_word_skips_whitespace` | Move by word on "foo  bar" | Lands on "bar" |
| `cursorset_add_cursor` | Add second cursor | Set has 2 cursors |
| `cursorset_merge_overlapping` | Add cursor at same position | De-duplicated to 1 |
| `cursorset_remap_after_insert` | Insert before cursors | Cursors shift forward |

### 3.6 `selection.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `selection_from_cursor_with_anchor` | Cursor at (0,5), anchor at (0,0) | Selection spans 0..5 |
| `selection_text_extraction` | Select range then get text | Correct substring |
| `selection_column_rect` | Column selection mode | Rectangular region selected |
| `selectionset_no_overlap` | Add overlapping selections | Merged into one |

### 3.7 `search.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `search_plain_finds_match` | Search "foo" in "foobar" | Match at (0,0..3) |
| `search_plain_case_insensitive` | Search "FOO" insensitive | Match found |
| `search_plain_no_match` | Search "xyz" in "abc" | Empty results |
| `search_regex_finds_pattern` | Search `\d+` in "abc123" | Match at "123" |
| `search_next_wraps_around` | At last match, next | Wraps to first |
| `search_incremental_update_after_edit` | Edit buffer after search | Matches updated |
| `search_replace_single` | Replace first match | One replacement |
| `search_replace_all` | Replace all matches | All replaced |

### 3.8 `encoding.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `encoding_detect_lf` | File with LF endings | `LineEnding::Lf` |
| `encoding_detect_crlf` | File with CRLF endings | `LineEnding::Crlf` |
| `encoding_detect_mixed` | Mixed endings | Majority wins or LF default |
| `encoding_preserve_on_save` | Open CRLF file, edit, save | File still has CRLF |

### 3.9 `recovery.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `recovery_swap_file_created_on_edit` | Edit a buffer | Swap file exists |
| `recovery_swap_file_deleted_on_save` | Save buffer | Swap file removed |
| `recovery_replay_restores_state` | Create swap, reload | Buffer matches pre-crash state |
| `recovery_corrupted_swap_returns_error` | Malformed swap file | `EditError::SwapFileCorrupted` |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `open_edit_save_roundtrip` | Open file → edit → save → re-read | Content matches |
| `large_file_open_and_scroll` | Open 100 MB synthetic file, read lines | No OOM, viewport within 500 ms |
| `multi_cursor_edit_consistency` | Add 3 cursors, type "x" | "x" inserted at all 3 positions |
| `undo_redo_full_session` | Series of edits → undo all → redo all | Final state matches |
| `search_replace_all_in_large_buffer` | 10,000 matches replaced | All replaced, correct content |
| `concurrent_buffer_access` | Multiple threads read buffer | No data races (compile-time via Send+Sync) |

---

## 5. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_insert_single_char` | < 1 µs |
| `bench_insert_1000_chars_sequential` | < 1 ms |
| `bench_delete_range_1000_bytes` | < 100 µs |
| `bench_undo_redo_100_steps` | < 1 ms |
| `bench_search_plain_100mb_file` | < 100 ms |
| `bench_search_regex_100mb_file` | < 500 ms |
| `bench_cursor_remap_1000_cursors` | < 1 ms |
| `bench_large_file_open_100mb` | viewport < 500 ms |

---

## 6. Coverage Target

**Minimum: 80%** line coverage on all source files in this crate.

Priority areas for full (near-100%) coverage:
- `edit.rs` — all edit command paths
- `undo.rs` — all branch/group scenarios
- `rope.rs` — boundary conditions (empty, single char, max size)
- `recovery.rs` — all error paths
