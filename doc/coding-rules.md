# SMASH — Coding Rules

This document defines the mandatory coding standards, conventions, and quality gates for the SMASH project. All contributors and AI-assisted tooling MUST follow these rules.

---

## 1. Language & Toolchain

| Item | Standard |
|---|---|
| Language | Rust (latest stable) |
| Edition | 2024 (or latest edition at time of development) |
| Minimum Supported Rust Version (MSRV) | Documented in `Cargo.toml` workspace `[workspace.package]` |
| Formatter | `rustfmt` (project `.rustfmt.toml` settings are authoritative) |
| Linter | `clippy` — all warnings must be resolved before merge |
| Build system | Cargo workspace (`Cargo.toml` at repo root) |

---

## 2. Project Structure Rules

- Each logical subsystem lives in its own crate under `crates/` (e.g., `smash-core`, `smash-lsp`).
- The binary entry point is `src/main.rs`; it only wires crates together — minimal logic.
- Platform-specific code MUST be isolated behind the `smash-platform` crate's abstraction traits.
- Integration and end-to-end tests go in the top-level `tests/` directory.
- Unit tests go in the same file as the code they test, inside a `#[cfg(test)] mod tests` block.

---

## 3. Naming Conventions

Follow the [Rust API Guidelines — Naming](https://rust-lang.github.io/api-guidelines/naming.html):

| Element | Convention | Example |
|---|---|---|
| Crates | `kebab-case` | `smash-core` |
| Modules | `snake_case` | `buffer_ops` |
| Types (struct, enum, trait) | `UpperCamelCase` | `EditCommand`, `BufferState` |
| Functions & methods | `snake_case` | `insert_char`, `apply_edit` |
| Constants & statics | `SCREAMING_SNAKE_CASE` | `MAX_BUFFER_SIZE` |
| Type parameters | Single uppercase or short `CamelCase` | `T`, `K`, `V`, `Iter` |
| Feature flags | `kebab-case`, descriptive (no placeholder words) | `collab-crdt`, `terminal-emulator` |

### Conversion method naming

| Prefix | Cost | Ownership |
|---|---|---|
| `as_` | Free / cheap | Borrow → borrow |
| `to_` | Expensive | Borrow → owned |
| `into_` | Variable | Owned → owned (consumes self) |

---

## 4. Code Style

### 4.1 Formatting
- Run `cargo fmt --all` before every commit. CI will reject unformatted code.
- Line width: 100 characters (configured in `.rustfmt.toml`).
- Use trailing commas in multi-line constructs.

### 4.2 Imports
- Group imports in this order, separated by a blank line:
  1. `std` / `core` / `alloc`
  2. External crates
  3. Workspace crates (`smash-*`)
  4. Crate-local (`crate::`, `super::`, `self::`)
- Prefer explicit imports over glob imports (`use module::*` is forbidden except in test modules and preludes).

### 4.3 Error Handling
- Use `Result<T, E>` for all fallible operations. Never `unwrap()` or `expect()` in production code.
- `unwrap()` and `expect()` are allowed ONLY in tests, examples, and provably-infallible contexts (with a comment explaining why).
- Define domain-specific error types per crate using `thiserror`.
- Use `anyhow` only at the binary entry point (`src/main.rs`) or integration tests, never in library crates.

### 4.4 Unsafe Code
- `unsafe` blocks MUST carry a `// SAFETY:` comment explaining the invariant being upheld.
- Minimize `unsafe` surface. Prefer safe abstractions. Every `unsafe` block requires code review approval from at least 2 reviewers.

### 4.5 Panics
- Library crates MUST NOT panic. Functions that can fail return `Result`.
- If a function has preconditions that would cause a panic, document them in a `# Panics` section in its doc comment.

### 4.6 Logging
- Use the `tracing` crate (not `println!` or `eprintln!`).
- Log levels: `error` (unrecoverable), `warn` (degraded), `info` (lifecycle events), `debug` (internal flow), `trace` (hot-path data).
- Never log sensitive data (file contents, credentials).

---

## 5. Documentation

- Every public item (`pub fn`, `pub struct`, `pub enum`, `pub trait`) MUST have a doc comment (`///`).
- Crate root (`lib.rs`) MUST have a `//!` module-level doc explaining purpose, usage, and examples.
- Doc comments MUST include:
  - A summary line.
  - `# Examples` section with a compilable example for non-trivial items.
  - `# Errors` section listing when the function returns `Err`.
  - `# Panics` section if applicable.
  - `# Safety` section for `unsafe fn`.
- Use `cargo doc --no-deps --document-private-items` to verify docs build without warnings.

---

## 6. Test-Driven Development (TDD)

### 6.1 TDD Workflow

All new functionality MUST follow a Red-Green-Refactor cycle:

1. **Red** — Write a failing test that describes the desired behavior before writing any implementation code.
2. **Green** — Write the minimum implementation code to make the test pass.
3. **Refactor** — Clean up the implementation while keeping all tests green.

No production code may be written without a corresponding test that was written first. Commits SHOULD reflect this cadence (test commit → implementation commit, or combined with clear commit message).

### 6.2 Test Organization

| Test Type | Location | Runs With |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` in the source file | `cargo test -p <crate>` |
| Integration tests | `tests/` at workspace root or crate-level `tests/` directory | `cargo test --test <name>` |
| Doc tests | Inline in `///` comments | `cargo test --doc` |
| Benchmarks | `benches/` directory (using `criterion`) | `cargo bench` |

### 6.3 Test Naming

```rust
#[test]
fn <unit>_<scenario>_<expected_behavior>() {
    // Arrange
    // Act
    // Assert
}
```

Examples:
```rust
#[test]
fn buffer_insert_at_start_prepends_text() { ... }

#[test]
fn lsp_client_timeout_returns_error() { ... }

#[test]
fn rope_delete_range_across_chunks_preserves_integrity() { ... }
```

### 6.4 Test Quality

- Tests MUST be deterministic — no reliance on timing, network, or filesystem ordering.
- Use `proptest` or `quickcheck` for property-based testing on core data structures (rope, CRDT).
- Use `mockall` or manual mocks via trait objects for external dependencies (LSP servers, filesystem).
- Each test tests ONE behavior. Avoid multi-assertion tests that obscure failure causes.
- Tests MUST clean up after themselves (use `tempfile` for filesystem tests).

---

## 7. Coverage Requirements

### 7.1 Targets

| Scope | Minimum Line Coverage |
|---|---|
| Core crates (`smash-core`, `smash-syntax`, `smash-lsp`, `smash-collab`, `smash-terminal`) | **≥ 80%** |
| Other library crates | **≥ 70%** |
| Binary entry point (`src/main.rs`) | Best effort (integration tests cover this) |

### 7.2 Measurement

- Coverage is measured with **`cargo-tarpaulin`** or **`cargo-llvm-cov`** in CI.
- Coverage reports are generated on every pull request. A PR that decreases coverage below the threshold MUST NOT be merged.
- Coverage badge is displayed in the repository README.

### 7.3 What Counts

- Unreachable code, debug-only code, and generated code MAY be excluded with `#[cfg(not(tarpaulin_include))]` or equivalent — with justification.
- Coverage of `unsafe` blocks is especially important; aim for 100% on unsafe paths.

---

## 8. Performance Rules

- Allocations in the hot path (input → render loop) MUST be minimized. Prefer stack allocation, arenas, or pre-allocated buffers.
- No blocking I/O on the main thread. All I/O goes through the async runtime.
- New code touching the render loop MUST be accompanied by a benchmark (`criterion`) demonstrating it meets the ≤ 16 ms frame budget.
- Profile before optimizing. Use `cargo flamegraph` or `perf` to identify bottlenecks.
- Startup-path code MUST be benchmarked with `hyperfine` to ensure ≤ 200 ms cold start.

---

## 9. Dependency Management

- New external dependencies require justification in the PR description.
- Prefer crates with:
  - Permissive license (MIT, Apache-2.0, or dual).
  - Active maintenance (commit within last 6 months).
  - No `unsafe` unless audited (check with `cargo-geiger`).
- Run `cargo deny check` in CI to enforce license and advisory policies.
- Pin dependency versions in `Cargo.lock` (committed to the repository).

---

## 10. Git & CI Rules

### 10.1 Branching
- `main` is the default branch; it must always build and pass all tests.
- Feature branches: `feat/<short-description>`
- Bug fix branches: `fix/<short-description>`
- Refactor branches: `refactor/<short-description>`

### 10.2 Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <summary>

[optional body]

[optional footer(s)]
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `ci`, `perf`, `chore`.

Scope is the crate name or area: `core`, `lsp`, `tui`, `config`, etc.

### 10.3 CI Pipeline (must all pass before merge)

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo doc --no-deps --document-private-items` (no warnings)
5. Coverage check (threshold gate)
6. `cargo deny check`
7. Build on all 3 platforms (Linux, macOS, Windows)

### 10.4 Pull Requests
- Every PR requires at least 1 approving review.
- PRs touching `unsafe` code require 2 approving reviews.
- All CI checks must pass before merge.
- Squash-merge to `main` with a conventional commit message.

---

## 11. API Design Guidelines

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html):

- Types eagerly implement common traits: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default` where appropriate.
- Use `From`/`Into` for conversions; implement on the most specific type.
- Errors implement `std::error::Error` + `Display` + `Debug`.
- Types are `Send + Sync` where possible.
- Structs have private fields with constructor functions (future-proofing).
- Sealed traits for extension points that must not be implemented downstream.
- Builder pattern for types with many optional configuration parameters.
- Functions validate their arguments; return `Result` rather than panicking.

---

## 12. Security

- No file path traversal — always canonicalize and validate paths.
- Remote/collaboration features MUST use TLS 1.3+.
- Plugin WASM sandboxing MUST NOT be bypassed; capabilities are declared in manifest.
- Credentials (SSH keys, tokens) MUST NOT be logged or persisted in plain text.
- Run `cargo audit` in CI to catch known vulnerabilities.
