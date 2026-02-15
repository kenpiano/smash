# smash-core — Module Design

## 1. Overview

`smash-core` is the foundational crate of SMASH. It owns the buffer data structure, undo/redo history, cursor/selection state, and all edit operations. Every other crate that manipulates text depends on `smash-core`.

**Phase**: 1 (MVP)

**Requirements Covered**: REQ-EDIT-001 – 024, REQ-PERF-010 – 022, REQ-NFR-001 – 002

---

## 2. Public API Surface

### 2.1 Core Types

```
Buffer          — owns a rope + metadata (encoding, line endings, dirty flag)
BufferId        — opaque handle identifying a buffer
Rope            — the underlying text data structure (wraps ropey or custom impl)
Position        — (line: usize, col: usize) — 0-based
ByteOffset      — absolute byte offset into the rope
Range           — (start: Position, end: Position)
EditCommand     — enum of all atomic edit operations
EditResult      — Result<EditOutcome, EditError>
```

### 2.2 Undo / Redo

```
UndoTree        — branching undo history (tree, not linear stack)
UndoNode        — a single undo entry (group of edits + cursor state)
UndoGroupId     — handle for grouping multiple edits into a single undo step
```

### 2.3 Cursor & Selection

```
Cursor          — a single cursor position + optional anchor (for selection)
CursorSet       — ordered set of cursors (multi-cursor support)
Selection       — derived from a Cursor with an anchor; (anchor, head) pair
SelectionSet    — collection of non-overlapping selections
```

### 2.4 Search

```
SearchQuery     — plain text or compiled regex
SearchMatch     — (range: Range, capture_groups: Option<...>)
SearchState     — current search context (query, matches, current index)
```

---

## 3. Internal Architecture

```
┌─────────────────────────────────────────────────┐
│                    Buffer                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │  Rope     │  │ Metadata │  │  LineEndingDB │  │
│  │  (text)   │  │ (enc,    │  │  (LF/CRLF/CR)│  │
│  │           │  │  dirty,  │  │               │  │
│  │           │  │  path)   │  │               │  │
│  └─────┬────┘  └──────────┘  └───────────────┘  │
│        │                                         │
│        ▼                                         │
│  ┌──────────────────────────────────────────┐    │
│  │            Edit Pipeline                  │    │
│  │                                           │    │
│  │  EditCommand → validate → apply to rope   │    │
│  │              → record in UndoTree         │    │
│  │              → emit EditEvent             │    │
│  └──────────────────────────────────────────┘    │
│                                                   │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ UndoTree │  │CursorSet │  │ SearchState   │  │
│  └──────────┘  └──────────┘  └───────────────┘  │
└─────────────────────────────────────────────────┘
```

### 3.1 Buffer

- Wraps a `Rope` plus metadata: file path, encoding, line ending style, dirty flag.
- Immutable-reference access for read paths (rendering, search indexing).
- Mutable edits go through the edit pipeline which validates, applies, and records history.

#### File Path Handling

All file paths are stored as `PathBuf` (OS-native representation) and support:

- **Spaces** in directory and file names (no special escaping — `PathBuf` handles them natively).
- **Non-ASCII / Unicode** names (e.g., CJK characters, accented Latin, emoji). Rust's `OsString`/`PathBuf` uses the OS-native encoding (UTF-8 on Linux/macOS, potentially WTF-16 on Windows via `OsStr`).
- **Normalisation**: No path normalisation is performed. Paths are stored exactly as opened. Comparison for "same file" detection uses canonical paths (`std::fs::canonicalize`) to handle symlinks and `..` components.
- **Display**: When displaying paths in the UI (title bar, file picker), paths are converted to UTF-8 via `Path::to_string_lossy()` so rendering never panics, but a lossy replacement character (`�`) may appear for rare non-UTF-8 byte sequences on Linux.
- **Swap / recovery files**: Derived file names (`.smash-swap`) are placed alongside the original file using the same directory, so they inherit the same filesystem encoding/capability.

### 3.2 Rope

- Primary data structure: a balanced tree of text chunks.
- O(log n) insert, delete, index-by-line, index-by-byte.
- Candidate implementation: `ropey` crate (proven, well-tested).
- If CRDT integration requires deeper control, a custom rope will be built on top of the same B-tree structure.

### 3.3 Edit Pipeline

Every mutation flows through a single `apply_edit` function:

```rust
pub fn apply_edit(&mut self, cmd: EditCommand) -> EditResult {
    // 1. Validate (range in bounds, encoding ok)
    // 2. Apply to rope
    // 3. Record inverse in UndoTree
    // 4. Update cursors (shift positions)
    // 5. Update dirty flag
    // 6. Emit EditEvent for subscribers (syntax, LSP, collab)
}
```

### 3.4 EditCommand Enum

```rust
pub enum EditCommand {
    Insert { pos: Position, text: String },
    Delete { range: Range },
    Replace { range: Range, text: String },
    IndentLines { lines: Vec<usize>, direction: IndentDirection },
    TransformCase { range: Range, case: CaseTransform },
    // Compound operations:
    Batch(Vec<EditCommand>),  // atomically applied, single undo entry
}
```

### 3.5 Undo Tree

- Each node stores the inverse operation and the cursor state before the edit.
- Branching: when a new edit is made after undoing, a new branch is created rather than discarding the previous future.
- `undo()` walks up to the parent node; `redo()` walks to the most recent child (or a user-selected branch).
- Grouping: multiple edits can be grouped into a single undo step (e.g., auto-close bracket = 2 inserts, 1 undo step).

#### Memory Management & Pruning

The undo tree does **not** grow without bound. The following policies prevent unbounded memory usage:

| Policy | Default | Configurable |
|---|---|---|
| **Max node count** | 10,000 nodes per buffer | `undo.max_nodes` |
| **Max memory** | 50 MB per buffer | `undo.max_memory_mb` |
| **Age limit** | 7 days since node creation | `undo.max_age_days` |

**Pruning algorithm** (runs after each new node insertion):

1. If the total node count or estimated memory exceeds the limit, prune the **oldest leaf branches** first (branches that are neither the current position nor ancestors of it).
2. Leaves whose age exceeds the age limit are pruned regardless of count/memory.
3. Pruning never removes the linear ancestor chain from the root to the current position — the active undo/redo path is always preserved.
4. When a subtree is pruned, its nodes are dropped and the parent's child list is updated. The memory is freed immediately (Rust ownership).
5. After a file is saved, nodes older than the save point may optionally be compacted: consecutive small edits are merged into a single coarser node to reduce per-node overhead while retaining the ability to undo back to the save point.

This ensures that even long editing sessions on a single buffer keep undo memory proportional to recent, relevant history.

### 3.6 Cursor & Multi-Cursor

- `CursorSet` maintains a sorted `Vec<Cursor>` with no overlaps.
- After each edit, all cursor positions are remapped based on the edit range/offset.
- Multi-cursor edits produce a `Batch` of `EditCommand`s, one per cursor.
- Operations: move (by char, word, line, paragraph, start/end), select (extend), add cursor at next match.

### 3.7 Search

- `SearchQuery::Plain(String)` — case-sensitive/insensitive toggle.
- `SearchQuery::Regex(regex::Regex)` — compiled once, reused.
- Search runs incrementally: when the buffer changes, only affected regions are re-scanned.
- Results stored as a sorted `Vec<SearchMatch>` for navigation (next/prev).

### 3.8 Large File Support

- Files > 10 MB: use memory-mapped I/O (`memmap2`) to create the initial rope without loading all bytes into heap.
- Files > 50 MB: signal to the syntax layer to disable or limit highlighting.
- Lazy line-ending detection: scan only as lines become visible.

### 3.9 Crash Recovery

- A swap file (`.smash-swap`) is written alongside the open file containing:
  - Original file hash.
  - A log of `EditCommand`s since last save.
- On startup, if a swap file exists, offer to recover.
- Swap file is updated on every edit or every 30 seconds (whichever is less frequent for busy edits).

---

## 4. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("position {0} is out of buffer bounds")]
    OutOfBounds(Position),

    #[error("invalid byte range {start}..{end}")]
    InvalidRange { start: ByteOffset, end: ByteOffset },

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("file I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("swap file corrupted")]
    SwapFileCorrupted,
}
```

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `ropey` | Rope data structure |
| `regex` | Regular expression search |
| `memmap2` | Memory-mapped I/O for large files |
| `thiserror` | Error type derivation |
| `encoding_rs` | Encoding detection/conversion (Phase 5) |

---

## 6. Performance Considerations

- Rope operations are O(log n) — no full-buffer copies on edit.
- Cursor remapping after edit is O(k log n) where k = number of cursors.
- Search indexing is incremental; full re-scan only on query change.
- Swap file writes are debounced and asynchronous (via channel to a tokio task).
- Memory budget: buffer metadata overhead < 2× file size (REQ-PERF-011).

---

## 7. Collaboration Hook Points

- `EditEvent` is the integration point for the CRDT layer.
- The `Buffer` exposes a `subscribe_edits() → Receiver<EditEvent>` channel.
- The CRDT layer can inject remote edits via `apply_edit()` with a flag marking them as remote (to skip re-broadcast).

---

## 8. Module File Layout

```
crates/smash-core/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public re-exports
    ├── buffer.rs           # Buffer struct, file I/O, metadata
    ├── rope.rs             # Rope wrapper (may just re-export ropey)
    ├── edit.rs             # EditCommand, EditResult, edit pipeline
    ├── undo.rs             # UndoTree, UndoNode, grouping
    ├── cursor.rs           # Cursor, CursorSet
    ├── selection.rs        # Selection, SelectionSet, column selection
    ├── search.rs           # SearchQuery, SearchState, incremental search
    ├── position.rs         # Position, ByteOffset, Range, conversions
    ├── encoding.rs         # Line-ending detection, encoding support
    ├── recovery.rs         # Swap file read/write, crash recovery
    └── error.rs            # EditError and related types
```
