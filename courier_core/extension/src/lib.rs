//! Extension primitives for builtin modules and plugin modules.
//!
//! # Builtin
//! A builtin module registers a [`Module`](crate::builtin::Module) into a [`Registry`](crate::builtin::Registry) and exposes its
//! functionality.
//! Builtin modules can operate native apis, such as providing concrete protocol implementations and so on.
//! Also, they are able to expose their own apis as a standard Rust library.
//! 
//! Builtin modules are registered at compile time. So no performance loss.
//! 
//! # Plugin
//! A plugin module is designed to be written in multiple programming language.
//! A valid plugin is compiled into consistent machine bytes or other form, executed by server.
//! But it's worth noting that plugins do not have the same abilities that `builtin`s have, for example,
//! providing custom protocol implementation. But it can operate apis offered by
//! main framework and `builtin`s(if exist).
//! 
//! Plugin modules are registered at bootstrap time. A slight performance loss.

pub mod builtin;
pub mod plugin;
