//! A composable, asynchronous logging framework.
//!
//! This crate provides a thread-safe logger built around a **flow**
//! abstraction, where each flow represents a destination for log output (e.g.,
//! console, file, or custom). Flows are composed **at compile time** via the
//! [`Stack`](flow::Stack) type, allowing multiple destinations to be chained
//! without dynamic dispatch.
//!
//! Log records are filtered at the call site, cross an internal channel and
//! are rendered on a dedicated worker thread: the configured [`Format`] step
//! writes the **complete** output line into a reusable buffer, and flows
//! write that line verbatim. The formatter has full control of the layout —
//! timestamps, levels, namespaces, payload placement, colours, JSON, … The
//! default formatter ([`DefaultFormatter`]) produces a
//! `[timestamp][LEVEL][namespace] message` layout.
//!
//! # Architecture
//!
//! ```text
//! macro → [level gate] → Record → Logger (bounded channel) → logger thread
//!       → timestamp → Format::format (cached timestamp, reusable buffer)
//!       → Flow chain writes the rendered line → output
//! ```
//!
//! Records do not carry a timestamp; the worker stamps each record when it is
//! dequeued, so the logging call site never reads a clock.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use logger::{Builder, Level};
//!
//! // Build and install the global logger.
//! Builder::new()
//!   .capacity(4096)
//!   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
//!   .build();
//!
//! // Logging macros are no-ops until a logger is installed.
//! logger::info!(namespace = "my_module", "Hello from the logger!");
//! ```
//!
//! # Features
//!
//! - **Level filtering** — each flow declares its minimum [`Level`]; records
//!   below that level are silently skipped, and a cheap call-site gate
//!   avoids formatting/allocating them at all.
//! - **Target routing** — records carry an optional `target`; a flow only
//!   accepts a record when its name matches the target.
//! - **Lock-free call site** — the record is handed to the worker with a
//!   single non-blocking channel send.
//! - **Single-pass rendering** — the worker renders each record exactly once
//!   into a reusable buffer; steady-state formatting allocates nothing and
//!   flows never re-format.
//! - **Full-layout formatters** — implement [`Format`] or pass a closure to
//!   control the entire output text.
//! - **Custom flows** — implement [`Flow`]; only `println` and `flush` are
//!   required.

mod builder;
mod config;
mod error;
pub mod flow;
pub mod flows;
mod format;
mod level;
mod logger;
mod record;
#[macro_use]
pub mod macros;

pub use builder::{Builder, Empty, NonEmpty};
pub use config::*;
pub use error::HandlingKind;
pub use flow::Flow;
pub use format::*;
pub use level::Level;
pub use logger::Logger;
pub use record::Record;
