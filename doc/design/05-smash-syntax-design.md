# smash-syntax — Module Design

## 1. Overview

`smash-syntax` provides syntax highlighting via Tree-sitter grammars. It performs incremental parsing on every buffer edit and outputs styled token spans consumed by the renderer.

**Phase**: 1 (MVP — 5 languages), Phase 2 (full 16+ languages)

**Requirements Covered**: REQ-SYN-001 – 011, REQ-PERF-022

---

## 2. Public API Surface

### 2.1 Core Types

```rust
HighlightEngine      — manages parsers and queries per language
LanguageId           — enum or string identifier for a language
HighlightConfig      — per-language highlight query + injection rules
HighlightSpan        — { range: ByteRange, scope: ScopeId }
ScopeId              — numeric ID mapping to a theme scope name (e.g., "keyword")
ScopeMap             — ScopeId ↔ scope name mapping
ParseState           — per-buffer Tree-sitter parse tree + metadata
EditNotification     — describes a buffer edit for incremental re-parse
```

### 2.2 Key Functions

```rust
/// Create a new engine, loading grammars from the grammars/ directory
pub fn new(grammar_dir: &Path) -> Result<HighlightEngine, SyntaxError>

/// Set up parsing for a specific buffer
pub fn attach_buffer(&mut self, buffer_id: BufferId, lang: LanguageId)
    -> Result<(), SyntaxError>

/// Notify of a buffer edit; re-parse incrementally
pub fn edit(&mut self, buffer_id: BufferId, edit: EditNotification)
    -> Result<(), SyntaxError>

/// Get highlight spans for a byte range (typically the visible viewport)
pub fn highlight(&self, buffer_id: BufferId, range: ByteRange)
    -> Vec<HighlightSpan>

/// Detect language from file extension or content
pub fn detect_language(path: &Path, first_line: &str) -> Option<LanguageId>
```

---

## 3. Internal Architecture

```
┌────────────────────────────────────────────────┐
│              HighlightEngine                   │
│                                                │
│  ┌─────────────────┐   ┌───────────────────┐  │
│  │ GrammarRegistry │   │ Per-Buffer State   │  │
│  │                 │   │                   │  │
│  │ lang → Grammar  │   │ buffer_id →       │  │
│  │ lang → HLQuery  │   │   ParseState {    │  │
│  │ lang → Injections│   │     tree,         │  │
│  │                 │   │     edit_log,     │  │
│  │                 │   │     lang          │  │
│  └─────────────────┘   │   }              │  │
│                         └───────────────────┘  │
│                                                │
│  ┌──────────────────────────────────────────┐  │
│  │         Incremental Parse Pipeline       │  │
│  │                                          │  │
│  │  EditNotification                        │  │
│  │    → tree.edit(InputEdit)                │  │
│  │    → parser.parse(tree, callback)        │  │
│  │    → extract changed ranges              │  │
│  │    → run highlight query on changed      │  │
│  │    → cache highlight spans               │  │
│  └──────────────────────────────────────────┘  │
└────────────────────────────────────────────────┘
```

### 3.1 Grammar Build Pipeline & `tree-sitter-cli`

Tree-sitter has two distinct components:

| Component | Crate / Tool | Role |
|---|---|---|
| **`tree-sitter-cli`** | CLI tool (`tree-sitter generate`) | Compiles `grammar.js` → C parser source. Used at **build time** only. |
| **`tree-sitter`** | Rust crate (`tree_sitter::Parser`) | Loads compiled parsers and runs incremental parsing at **runtime**. |

SMASH uses the **`tree-sitter` Rust crate** as the runtime backend. `tree-sitter-cli` is used only in the grammar build pipeline:

```
grammar.js  ──[tree-sitter generate]──▶  parser.c / scanner.c
                                              │
                                     [cc / build.rs]
                                              │
                                              ▼
                                    parser.so / .dylib / .dll
                                              │
                              (placed in grammars/ directory)
                                              │
                              [smash-syntax loads at runtime]
```

**Build options** (evaluated at compile time, not runtime):
1. **Vendored grammars** (default): The `grammars/` directory ships pre-compiled shared libraries for all supported languages. The CI pipeline uses `tree-sitter-cli` + a C compiler to rebuild them from source on each platform.
2. **Static linking**: For distribution simplicity, grammars can alternatively be compiled into the SMASH binary via Cargo build scripts using `cc` crate (no runtime `.so` loading needed). This is the preferred approach for release builds.
3. **User-added grammars**: Users can add any publicly available tree-sitter grammar for languages not bundled by default (see §3.2).

### 3.2 Adding Unsupported Languages

The `tree-sitter` Rust crate is **language-agnostic** — it is a generic incremental parsing engine that knows nothing about specific languages. Any language with a published tree-sitter grammar can be added to SMASH without code changes. There are **200+ community-maintained grammars** on GitHub (e.g., Elixir, Haskell, Kotlin, Swift, Zig, Lua, OCaml, Scala, Dart, etc.).

To add a new language, a user provides:

| File | Purpose | Where to find |
|---|---|---|
| Compiled parser (`.so`/`.dylib`) | The parser itself | Build from the grammar's repo using `tree-sitter-cli` + C compiler |
| `highlights.scm` | Highlight query (maps AST nodes → scopes) | Usually included in the grammar repo under `queries/` |
| `injections.scm` *(optional)* | Language injection rules | Same location, if the language embeds other languages |

**User workflow** to add e.g. Elixir:

```sh
# 1. Clone the grammar
git clone https://github.com/elixir-lang/tree-sitter-elixir

# 2. Build the shared library (requires tree-sitter-cli + C compiler)
cd tree-sitter-elixir
tree-sitter generate
cc -shared -o elixir.so -I src src/parser.c src/scanner.c

# 3. Install into SMASH's user grammar directory
mkdir -p ~/.config/smash/grammars/elixir
cp elixir.so ~/.config/smash/grammars/elixir/
cp queries/highlights.scm ~/.config/smash/grammars/elixir/

# 4. Register the file extension in config
# ~/.config/smash/config.toml
# [languages.elixir]
# extensions = ["ex", "exs"]
# grammar = "elixir"
```

SMASH will discover the grammar at startup (or on config reload) and make it available for syntax highlighting — no recompilation of SMASH needed.

> **Future improvement (Phase 5)**: A `smash grammar install <name>` CLI subcommand could automate this by downloading pre-built grammar packages from a registry.

### 3.3 Grammar Registry

- Loads compiled Tree-sitter grammars — either statically linked or dynamically from `.so` / `.dylib` / `.dll` files in the bundled `grammars/` directory and the user directory `~/.config/smash/grammars/`.
- Each language has: `Grammar` (the parser), `HighlightQuery` (`.scm` file), and optional `InjectionQuery` (for embedded languages like JS in HTML).
- User-installed grammars in `~/.config/smash/grammars/<lang>/` take precedence over bundled grammars of the same name (allows upgrading a bundled grammar).
- Loaded lazily on first use to minimize startup time.

### 3.4 Incremental Parsing

1. On buffer edit, receive `EditNotification { start_byte, old_end_byte, new_end_byte, start_point, old_end_point, new_end_point }`.
2. Call `tree.edit()` on the existing parse tree.
3. Re-parse with `parser.parse()` using a callback that reads from the rope.
4. Tree-sitter only re-parses the changed subtree — O(edit size × log n).

### 3.5 Highlight Query Execution

- Runs `QueryCursor` over the visible byte range only (not the entire file).
- Maps each captured node to a `ScopeId` via the highlight query's capture names.
- Output: `Vec<HighlightSpan>` sorted by start byte.

### 3.6 Language Detection

Priority order:
1. User override in configuration.
2. File extension mapping (e.g., `.rs` → Rust).
3. First-line pattern (e.g., `#!/bin/bash` → Shell).
4. Fallback: plain text (no highlighting).

### 3.7 Large File Handling

- Files > 50 MB: highlight only the visible viewport + a configurable margin (default: ±500 lines).
- The parse tree is still maintained for the full file (Tree-sitter is efficient), but query execution is range-limited.

### 3.8 Language Injection

- Supported for embedded languages (e.g., `<script>` in HTML, code blocks in Markdown).
- `InjectionQuery` defines which tree nodes contain embedded content and what language they use.
- The engine manages a separate `ParseState` per injection region.

---

## 4. Supported Languages (Launch)

| Language | Grammar Source | Phase |
|---|---|---|
| Rust | `tree-sitter-rust` | 1 |
| Python | `tree-sitter-python` | 1 |
| JavaScript | `tree-sitter-javascript` | 1 |
| TypeScript | `tree-sitter-typescript` | 1 |
| JSON | `tree-sitter-json` | 1 |
| C | `tree-sitter-c` | 2 |
| C++ | `tree-sitter-cpp` | 2 |
| Go | `tree-sitter-go` | 2 |
| Java | `tree-sitter-java` | 2 |
| Ruby | `tree-sitter-ruby` | 2 |
| Shell | `tree-sitter-bash` | 2 |
| Markdown | `tree-sitter-markdown` | 2 |
| YAML | `tree-sitter-yaml` | 2 |
| TOML | `tree-sitter-toml` | 2 |
| HTML | `tree-sitter-html` | 2 |
| CSS | `tree-sitter-css` | 2 |

---

## 5. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("grammar not found for language: {0}")]
    GrammarNotFound(String),

    #[error("failed to load grammar: {0}")]
    GrammarLoad(String),

    #[error("highlight query error: {0}")]
    QueryError(String),

    #[error("parser error for buffer {0}")]
    ParseFailed(BufferId),
}
```

---

## 6. Dependencies

| Crate | Purpose |
|---|---|
| `tree-sitter` | Core runtime parsing library |
| `tree-sitter-highlight` | Highlight query execution |
| `tree-sitter-cli` | Grammar compilation (**build-time only**, not a runtime dependency) |
| `cc` | Compile grammar C sources in build scripts |
| `libloading` | Dynamic grammar loading (dev builds / user-added grammars) |
| `smash-core` | Buffer types, EditNotification |

---

## 7. Performance Considerations

- Incremental parse: only changed subtree is re-parsed (~microseconds for typical edits).
- Highlight query: only visible range is queried.
- Grammar loading is lazy — no startup cost for unused languages.
- Parse trees are stored per-buffer; dropped when buffer is closed.

---

## 8. Module File Layout

```
crates/smash-syntax/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── engine.rs           # HighlightEngine
    ├── grammar.rs          # GrammarRegistry, dynamic loading
    ├── parse_state.rs      # Per-buffer parse tree management
    ├── highlight.rs        # Query execution, HighlightSpan
    ├── scope.rs            # ScopeId, ScopeMap
    ├── detect.rs           # Language detection
    ├── injection.rs        # Language injection support
    └── error.rs            # SyntaxError
```
