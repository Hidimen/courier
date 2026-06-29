# Rust Doc Comments Style in Project

This document defines the standard for all doc comments in the `courier`
workspace. Every public item **must** have a doc comment following these
rules.

## Structure

A standard doc comment consists of three parts:

1. **Description** — a one-sentence summary of what the item does, providing
   a clear purpose for users.
2. **Detailed information** — one or more markdown sections (chapters) with
   headings like `# Examples`, `# Panics`, etc.
3. **Note** (optional) — additional attention points using `**Note**:`.

> It is worth noting that an example is **always** needed unless the
> signature alone is self-explanatory. Each example **should pass** as a
> doc test.

### Module-level docs (`//!`)

Module and crate-level docs use `//!` and follow the same structure:

```rust
//! A one-sentence description of the module.
//!
//! Additional paragraphs with architectural context, design rationale,
//! or a birds-eye view of how the pieces fit together.
//!
//! # Quick start (or # Example)
//!
//! ```rust
//! // A working, minimal example.
//! ```
```

## Format

### Sentence style

- Every sentence ends with a dot (`.`).
- Use backticks for code identifiers: [`Level`], `true`, `Fn(Record) -> Record`.
- Use `[` `]` for intra-doc links: [`Flow`](crate::flow::Flow).

### Description

| Item type           | Starts with                | Example                                              |
|---------------------|----------------------------|------------------------------------------------------|
| Function / method   | Verb (third-person)        | `Creates a new Builder with default settings.`    |
| Struct              | Noun or v-ing form         | `A builder for constructing a Logger.`            |
| Enum                | Noun                       | `Represents the severity level of a log record.`     |
| Trait               | Noun / verb / v-ing        | `Formats log records before they are dispatched.`     |
| Macro               | Verb / noun / v-ing        | `Logs a message at the Info level.`                   |
| Type alias          | Noun                       | `An owning iterator over the entries of a HashMap.` |
| Module              | Noun / v-ing               | `Composable flow implementations.`                    |

### Example code blocks

- Keep examples minimal — show only what is necessary to understand usage.
- Obey the project's Rust code style.
- Prefer `no_run` over `ignore` unless the example cannot compile. Attributes
  in order of preference:

| Attribute        | When to use                                                    |
|------------------|----------------------------------------------------------------|
| _(none)_         | Default — example compiles and runs as a test.                 |
| `no_run`         | Compiles but does not run (e.g. it spawns threads, needs I/O). |
| `should_panic`   | Expected to panic at runtime.                                  |
| `ignore`         | Cannot even compile (e.g. macros needing global state).        |
| `compile_fail`   | Expected to fail compilation (use sparingly).                  |
| `edition2018`    | Needs a specific edition (rare).                               |

### Notes

Use Markdown emphasis for notes placed after the description and before the
chapters:

```rust
/// **Note**: The logger must be installed before calling this macro.
```

## Chapters

### `# Examples` (required in nearly all cases)

Show a minimal but complete usage. The reader should be able to copy,
paste, and understand it without extra context.

- Wrap imports and setup in the example — be self-contained.
- Use `# ` to hide boilerplate that distracts from the key usage.
- Prefer the plural `# Examples` when there are multiple examples;
  `# Example` for a single one is also acceptable.

```rust
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use logger::Level;
///
/// assert!(Level::Trace < Level::Info);
/// assert_eq!(Level::Error.to_string(), "ERROR");
/// ```
```

### `# Panics`

Include **only** when the function/method may panic. Clearly state the
exact condition that triggers the panic:

```rust
/// # Panics
///
/// Panics if a logger has already been installed via a previous call to
/// [`build`](Builder::build).
```

### `# Errors`

Include when the function/method returns a `Result`. Explain each error
variant and what causes it:

```rust
/// # Errors
///
/// Returns `Err("Capacity is unknown")` if [`capacity`](Builder::capacity)
/// was not called before building.
```

### `# Safety`

Include when the function/method is `unsafe`. Mention **all** preconditions
the caller must uphold for the call to be sound:

```rust
/// # Safety
///
/// The caller must ensure that `ptr` is non-null, properly aligned, and
/// points to valid memory for the lifetime `'a`.
```

### `# Returns`

Include when the return value needs explanation beyond what the type
signature conveys (e.g. the meaning of an `Option` or a sentinel value):

```rust
/// # Returns
///
/// Returns `None` if the channel is full and the record could not be sent.
```

### Other chapters

- `# Type parameters` — explain generic parameters when they aren't obvious.
- `# Fields` — document public struct fields (in addition to or instead of
  doc comments on each field).
- `# Implementation notes` — for non-obvious internal logic or performance
  characteristics.

## Documenting sub-items

### Struct fields

```rust
pub struct Record {
  /// Unix timestamp in seconds when the record was created.
  pub timestamp: i64,
  /// Severity level of the record.
  pub level: Level,
  /// Optional flow name for targeted routing.
  pub target: Option<&'static str>,
}
```

### Enum variants

```rust
pub enum HandlingKind {
  /// An unrecoverable error that should cause the logger to panic.
  Fuse(String),
  /// A recoverable error that the logger should silently ignore.
  Ignore,
}
```

## Hiding items from documentation

| Mechanism                   | Scope                  | When to use                          |
|-----------------------------|------------------------|--------------------------------------|
| `#[doc(hidden)]`            | Single item            | Hide one re-export or helper.        |
| `#![doc(hidden)]`           | Entire crate/module    | Hide an internal module from docs.   |
| `[lib] doc = false`         | Entire lib target      | Suppress doc generation for a crate. |

Prefer `#[doc(hidden)]` for surgical hiding; use `doc = false` only for
pure facade crates whose sub-crates carry the real documentation.

## Quick-reference checklist

Before marking a PR ready, verify each public item has:

- [ ] One-sentence description with the correct part-of-speech.
- [ ] Every sentence ends with `.`.
- [ ] At least one `# Examples` (or `# Example`) section with a doc test.
- [ ] `# Panics` section if the function may panic.
- [ ] `# Errors` section if it returns `Result`.
- [ ] `# Safety` section if it is `unsafe`.
- [ ] Backticks around code identifiers.
- [ ] Intra-doc links (`[` `]`) for related items.
- [ ] `rust,no_run` or `rust,ignore` if the doc test can't run as-is.
