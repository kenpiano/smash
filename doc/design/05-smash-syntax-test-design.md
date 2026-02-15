# smash-syntax — Test Design

## 1. Overview

Test strategy for `smash-syntax`. Requires ≥ 80% coverage (core crate).

---

## 2. Test Categories

| Category | Location | Tool |
|---|---|---|
| Unit tests | `src/*.rs` → `#[cfg(test)] mod tests` | `cargo test` |
| Integration tests | `crates/smash-syntax/tests/` | `cargo test` |
| Benchmarks | `crates/smash-syntax/benches/` | `criterion` |

---

## 3. Unit Test Plan

### 3.1 `detect.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `detect_rust_by_extension` | Path "main.rs" | `LanguageId::Rust` |
| `detect_python_by_extension` | Path "script.py" | `LanguageId::Python` |
| `detect_shell_by_shebang` | First line `#!/bin/bash` | `LanguageId::Shell` |
| `detect_unknown_extension` | Path "data.xyz" | `None` |
| `detect_no_extension_no_shebang` | Path "Makefile", no shebang | Fallback heuristic or None |

### 3.2 `grammar.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `grammar_load_rust` | Load Rust grammar | Valid Grammar object |
| `grammar_load_nonexistent` | Load "foobar" grammar | `SyntaxError::GrammarNotFound` |
| `grammar_lazy_load` | Request same grammar twice | Loaded once, cached |

### 3.3 `highlight.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `highlight_rust_keyword` | Parse `fn main()` | "fn" → keyword scope |
| `highlight_rust_string` | Parse `"hello"` | String literal scope |
| `highlight_range_only_viewport` | 1000-line file, query lines 500-510 | Only spans in range returned |
| `highlight_empty_buffer` | Empty content | Empty span list |

### 3.4 `parse_state.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `parse_initial_creates_tree` | Parse "let x = 1;" | Valid tree, root node exists |
| `parse_incremental_after_insert` | Initial parse, insert char, re-parse | Tree updated, no errors |
| `parse_incremental_after_delete` | Delete range, re-parse | Tree updated correctly |
| `parse_handles_syntax_errors` | Parse "let = ;" (invalid) | Tree has ERROR nodes, no crash |

### 3.5 `injection.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `injection_html_script` | HTML with `<script>` tag | JS content highlighted as JS |
| `injection_markdown_code_block` | Markdown with ` ```rust ` block | Rust code highlighted |

### 3.6 `scope.rs`

| Test Name | Scenario | Expected |
|---|---|---|
| `scope_map_insert_and_lookup` | Register "keyword", look up by ID | Correct name returned |
| `scope_map_unknown_id` | Look up unregistered ID | `None` |

---

## 4. Integration Tests

| Test Name | Scenario | Requirements |
|---|---|---|
| `full_highlight_pipeline_rust` | Load grammar → parse Rust file → get spans | Spans cover keywords, strings, comments |
| `incremental_edit_cycle` | Parse → insert → re-highlight | Changed spans correct, unchanged stable |
| `highlight_16_languages` | Parse sample file for each supported lang | No panics, spans non-empty |
| `large_file_viewport_highlight` | 100 MB file, highlight viewport | Completes in < 50 ms |

---

## 5. Benchmarks

| Benchmark | Target |
|---|---|
| `bench_initial_parse_1kb_rust` | < 5 ms |
| `bench_initial_parse_100kb_rust` | < 50 ms |
| `bench_incremental_parse_single_char_insert` | < 500 µs |
| `bench_highlight_viewport_100_lines` | < 2 ms |
| `bench_grammar_load_first_time` | < 50 ms |

---

## 6. Coverage Target

**Minimum: 80%** line coverage.

Priority for near-100%:
- `highlight.rs` — all scope mapping paths
- `parse_state.rs` — incremental parse edge cases
- `detect.rs` — all extension/shebang branches
