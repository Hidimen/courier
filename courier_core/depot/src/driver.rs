//! Event loops that drive one endpoint at a time.
//!
//! A driver owns one endpoint — a [`stream`] connection or a [`datagram`]
//! socket — and maps the events it receives onto the protocol model of the
//! matching transport.

pub mod datagram;
pub mod stream;

pub use datagram::drive_datagram;
pub use stream::drive_stream;
