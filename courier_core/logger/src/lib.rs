//! A composable, asynchronous logging framework.
//!
//! This crate provides a thread-safe logger built around a **flow** abstraction,
//! where each flow represents a destination for log output (e.g., console,
//! file, or custom). Flows are composable via the [`Stack`] type, allowing
//! multiple destinations to be chained together.
//!
//! Log records pass through a user-configurable [`Format`] step before being
//! dispatched to the registered flows. The default formatter
//! ([`DefaultFormatter`]) produces a `[timestamp][LEVEL] message` layout.
//!
//! # Architecture
//!
//! ```text
//! macro → Logger (channel) → logger thread → Format → Flow(s) → output
//! ```
//!
//! # Quick start
//!
//! ```rust,no_run
//! use logger::{Builder, Level};
//!
//! // Build and install the global logger.
//! Builder::new()
//!     .capacity(4096)
//!     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
//!     .build()
//!     .expect("Failed to install logger");
//!
//! // Logging macros are available once the logger is installed.
//! logger::info!("my_module", "Hello from the logger!");
//! ```
//!
//! # Features
//!
//! - **Level filtering** — each flow declares its minimum [`Level`]; records
//!   below that level are silently skipped.
//! - **Target routing** — records carry an optional `target`; a flow only
//!   accepts a record when its name matches the target.
//! - **Custom formatters** — implement [`Format`] or pass a closure.
//! - **Custom flows** — implement [`Flow`]; only `println` and `flush` are
//!   required.

mod builder;
mod error;
pub mod flow;
pub mod flows;
mod format;
mod level;
mod logger;
mod record;
#[macro_use]
pub mod macros;

pub use builder::{Builder, DefaultFormatter, Empty, NonEmpty};
pub use error::HandlingKind;
pub use flow::Flow;
pub use format::Format;
pub use level::Level;
pub use logger::Logger;
pub use record::Record;
