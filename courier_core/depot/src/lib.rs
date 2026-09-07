//! Server instance.
//!
//! [`Depot`] means a server instance, which combines a custom transport, a custom protocol
//! and a custom pipeline.
//!
//! # Structure
//! ```
//! transport   protocol   relay   event
//!     \          \         /       /
//!      \          \       /       /
//!       \          \     /       /
//!        \          \   /       /
//! ---------------------------------------
//! |                                     |
//! |              Depot                  |
//! |                                     |
//! ---------------------------------------
//!              /          \
//!             /            \
//!            /              \
//!           /                \
//!       run_stream          run_datagram
//! ```
//!
//! The instance support termination signal from keyboard or OS, which means graceful shutdown is supported.
//! When launching a depot instance, a provider is a necessary part, for it defines
//! transport(stream or datagram), protocol(core parts don't provide) and pipeline
//! (needs to be determined at compile time). Besides that, [`config`] should always exists.
//!
//! # Build
//! Depot can be created via [`Builder`]. [`StreamProvider`] and [`DatagramProvider`] should
//! always be implemented by builtin modules.
//!
//! # Running
//! When a new connection comes, depot will accept it, getting raw frame data.
//! Then depot starts an async task, where frame data is decoded into specific
//! data structure defined by protocol provider. Next, it flows into pipeline system([`relay`]),
//! do more operations. Finally, data is encoded into frame, sended to peer.
//!
//! # Shutdown
//! When a termination signal is received, depot accepts no new data. Connections with no request
//! in flight are closed at once, while the in-flight ones are given `server.shutdown_timeout`
//! (10 seconds by default) to write their response; tasks that are still running when that grace
//! period elapses are aborted. Signals received while the shutdown is in progress are ignored, so
//! the grace period is what bounds it — no signal can turn a graceful shutdown into an abrupt one.
//! Once every connection task has finished, the host flushes the event bus and the logger before
//! the process exits.

mod builder;
pub mod config;
pub mod depot;
pub mod error;
mod handle;
mod provider;
pub mod run;
mod signal;

pub use builder::Builder;
pub use depot::Depot;
pub use error::BuildError;
pub use handle::*;
pub use provider::*;
pub use signal::wait_for_termination;
