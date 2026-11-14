//! Extension primitives for builtin modules.
//!
//! A builtin module registers a [`Module`] into a [`Registry`] and exposes its
//! functionality as a [`Capability`]. The runtime a capability is served on is
//! a type parameter, so this crate does not depend on the concrete server.

pub mod builtin;
