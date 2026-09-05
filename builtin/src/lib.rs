//! Builtins for project.
//!
//! This lib provides apis exposed by builtin modules installed.

mod builtin_module;

pub use builtin_module::*;

macro_builtin_build::module_register!("../assets/registry.txt");
