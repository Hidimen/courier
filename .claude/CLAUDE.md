# CLAUDE.md

## Project Overview

**Courier** is a Rust async network server framework (v0.1.0, Apache-2.0). It provides composable transport, protocol, logging, and service middleware abstractions on top of Tokio.

## Build & Test

```bash
cargo build                    # build everything
cargo build -p <crate>         # build a specific crate
cargo test                     # run all tests (currently only logger has tests)
cargo test -p logger           # run logger tests
cargo test --doc               # run doc tests
cargo fmt --all -- --check     # check formatting
cargo clippy --all-features    # lint everything
```

## Code Style

### Rustfmt (`rustfmt.toml`)

- **Edition**: 2024
- **Indent**: 2 spaces, no hard tabs
- **Max width**: 80 columns
- **Imports**: reorder + group (`reorder_imports = true`, `reorder_modules = true`)
- **Fn params**: compressed layout (`fn_params_layout = "Compressed"`)
- **Small heuristics**: `Max` (aggressively inline small items)
- **Shorthand**: `use_field_init_shorthand = true`, `use_try_shorthand = true`
- **Match**: trailing comma on blocks
- **Newlines**: Unix (`\n`)
- **Parens**: remove nested parens where possible

Always run `cargo fmt` before committing.

### Doc Comments (`docs/comments-style.md`)

Every **public item** MUST have a doc comment following a strict three-part structure:

1. **One-sentence description** — starts with specific part-of-speech:
   - Functions: verb (e.g. "Creates a new Builder.")
   - Structs: noun or v-ing (e.g. "A builder for constructing a Logger.")
   - Enums: noun (e.g. "Represents the severity level of a log record.")
   - Traits: noun/verb/v-ing (e.g. "Formats log records before they are dispatched.")
   - Macros: verb/noun/v-ing (e.g. "Logs a message at the Info level.")
2. **Detailed chapters** — at minimum `# Examples` with a working doc test
3. **Optional note** — `**Note**: ...` for important caveats

Every sentence ends with `.`. Code identifiers in backticks. Intra-doc links with `[` `]`.

Required chapters by item type:
- `# Examples` — **always** (prefer `# Examples` plural; `# Example` for a single one is also ok)
- `# Panics` — if the function may panic
- `# Errors` — if it returns `Result`
- `# Safety` — if it is `unsafe`
- `# Returns` — if the return value needs explanation beyond the type signature

Example code attributes in preference order: `(none)` > `no_run` > `should_panic` > `ignore` > `compile_fail`. Use `no_run` when the example spawns threads or does I/O but still compiles.

Module-level docs use `//!` with the same structure.

### General Rust Conventions

- Prefer `#[doc(hidden)]` for surgical hiding of internal items. Use `[lib] doc = false` only for pure facade crates (like `courier`).
- Use `thiserror` for error types — the project workspace already depends on `thiserror 2.0.18`.
- Use `async-trait` (`0.1.89`) for traits with async methods.
- Tokio (`1.52.1`) is the async runtime — use `#[tokio::test]` for async tests.
- Type-state builder pattern is used in `logger::Builder<F, M>` — follow that pattern for new builders.

## Architecture Conventions

### Crate Naming & Exports

- Core crates live under `courier_core/`.
- The `courier` facade re-exports core crates. New core crates should be added to both `courier/Cargo.toml` dependencies and `courier/src/lib.rs` re-exports.
- Each core crate exposes its public API via `pub use` in `lib.rs` and `pub mod` for sub-modules.

### Error Handling

- `depot` errors wrap protocol errors via `ProtocolErrorWrapper<E>` newtype.
- Logger errors use `HandlingKind::Fuse` (unrecoverable, logger panics) and `HandlingKind::Ignore` (recoverable, silently skipped).
- Prefer `thiserror` for new error types. Follow the `ProtocolError` severity pattern for protocol-level errors.

## Testing

- Currently only `logger` has tests (unit tests in each module + integration tests in `logger/tests/`).
- New code in other crates should add tests.
- Use `#[cfg(test)]` modules for unit tests embedded in source files.
- Doc tests in `# Examples` sections serve as both documentation and tests — ensure they compile and run.

## Misc

- Rust edition 2024, minimum rustc 1.91.
- Workspace resolver v3.
- Git user: `SuiBian9516`, branch: `master`.
- Tabs: 2 spaces, no hard tabs, max 80 columns.
