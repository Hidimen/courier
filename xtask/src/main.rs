//! Build tooling for the courier project, exposed as `cargo xtask`.
//!
//! This crate is a port of the standalone `courier-build` tool (the Rust port
//! of the former Python build scripts that used to live in `build/` of the
//! courier repository, later integrated into the carrier workspace as
//! `crates/build`). Unlike the carrier integration, it does **not** mirror the
//! project into a `.courier` temporary build directory: every command works
//! directly on the real project tree.
//!
//! - [`register`] discovers the built-in modules under `builtins/` and
//!   generates the artifacts that wire the selected ones into the workspace:
//!   `builtin/Cargo.toml` (the `# AUTOGEN DEP #` section) and
//!   `builtin/assets/registry.txt` (read by the `module_register!` proc macro);
//! - [`git_cloner`] clones git placeholder modules into `build_cache/`;
//! - [`commands`] orchestrates `build`, `run`, `clean` and `generate`.
//!
//! No modules are registered by default: pass `--with-modules` to pick some or
//! `--with-all-modules` to take every discovered module. Commands that write
//! generated files roll them back to the empty state when they finish, unless
//! `--keep` is passed.

mod backend;
mod checker;
mod cli;
mod commands;
mod define;
mod git_cloner;
mod module;
mod paths;
mod placeholder;
mod register;

pub use module::BuiltinModule;
pub use placeholder::Placeholder;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
  cli::Cli::parse().execute().await
}
