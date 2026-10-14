//! Implementation shared by the build-tool subcommands.
//!
//! Port of the standalone `courier-build` `commands.rs` (former `build.py`,
//! `run.py`, and `clean.py`). The carrier version ran every step against a
//! `.courier` temporary mirror of the project; here every step runs against
//! the real project tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use loglet::info;

use crate::paths::{cache_path, output_path, project_root, target_path};
use crate::{BuiltinModule, checker, register};

/// The cargo package that owns the `courier` executable.
const COURIER_PACKAGE: &str = "waybill";

/// Regenerates the builtin artifacts with the given module selection.
///
/// `with_all` takes every discovered module; otherwise `names` picks the
/// modules to register, and an empty `names` registers none. The artifacts are
/// reset first, so a selection can never leave a stale registry behind —
/// [`register::generate_files`] alone skips empty selections.
///
/// # Errors
///
/// Returns an error if the `builtins` directory cannot be read, if a module
/// name is unknown, or if the generated files cannot be written.
pub fn generate_selection(
  root: &Path, with_all: bool, names: &[String],
) -> anyhow::Result<()> {
  let modules = selected_modules(root, with_all, names)?;
  register::clean_files(root)?;
  register::generate_files(root, &modules)
}

/// Runs `cargo` with `args` from the project root and fails on a non-zero
/// exit status.
///
/// # Errors
///
/// Returns an error if `cargo` cannot be spawned or exits with a non-zero
/// status.
pub fn run_cargo(root: &Path, args: &[String]) -> anyhow::Result<()> {
  let status = Command::new("cargo").args(args).current_dir(root).status()?;
  if !status.success() {
    anyhow::bail!("`cargo {}` failed with status {status}", args.join(" "));
  }

  Ok(())
}

/// Copies the built `courier` executable into `output`.
///
/// # Errors
///
/// Returns an error if the output directory cannot be created or the
/// executable cannot be copied.
pub fn copy_executable(
  root: &Path, mode: &str, output: &Path,
) -> anyhow::Result<()> {
  fs::create_dir_all(output).context("Cannot create output dir")?;

  let executable = if cfg!(windows) { "courier.exe" } else { "courier" };
  let source = target_path(root).join(mode).join(executable);
  let destination = output.join(executable);
  fs::copy(&source, &destination)?;
  info!("Copied {} to {}", source.display(), destination.display());

  Ok(())
}

/// Removes `path` and everything under it, doing nothing when it is absent.
///
/// # Errors
///
/// Returns an error if the directory cannot be removed.
pub async fn remove_dir(path: &Path) -> anyhow::Result<()> {
  if fs::exists(path)? {
    tokio::fs::remove_dir_all(path).await?;
  }

  Ok(())
}

/// Rolls the generated artifacts back to their empty state.
///
/// # Errors
///
/// Returns an error if the generated files cannot be rewritten.
pub fn rollback_generated(root: &Path) -> anyhow::Result<()> {
  register::clean_files(root)?;
  info!("Generated files rolled back; pass `--keep` to keep them");

  Ok(())
}

/// Builds the project and copies the executable into `output`.
///
/// Runs the generator with the selection, invokes `cargo build -p waybill`
/// (with `--release` when `mode` is `"release"`), then copies the `courier`
/// executable into `output`. Unless `keep` is set, the generated artifacts are
/// rolled back afterwards.
///
/// # Errors
///
/// Returns an error if generation fails, if the build exits with a non-zero
/// status, or if the executable cannot be copied.
pub async fn build_project(
  root: &Path, mode: &str, output: &Path, with_all: bool,
  with_modules: &[String], args: &[String], keep: bool,
) -> anyhow::Result<()> {
  info!("Building project...");
  generate_selection(root, with_all, with_modules)?;

  let mut cargo_args =
    vec!["build".to_string(), "-p".to_string(), COURIER_PACKAGE.to_string()];
  if mode == "release" {
    cargo_args.push("--release".to_string());
  }
  cargo_args.extend_from_slice(args);
  run_cargo(root, &cargo_args)?;

  copy_executable(root, mode, output)?;

  if !keep {
    rollback_generated(root)?;
  }

  Ok(())
}

/// Generates the builtin artifacts and runs the project.
///
/// Runs the generator with the selection, then invokes `cargo run -p waybill`
/// from the project root, forwarding `args` to the spawned executable. Unless
/// `keep` is set, the generated artifacts are rolled back after the program
/// exits.
///
/// # Errors
///
/// Returns an error if generation fails or if `cargo run` exits with a
/// non-zero status.
pub async fn run_project(
  root: &Path, with_all: bool, with_modules: &[String], args: &[String],
  keep: bool,
) -> anyhow::Result<()> {
  info!("Running project...");
  generate_selection(root, with_all, with_modules)?;

  let mut cargo_args =
    vec!["run".to_string(), "-p".to_string(), COURIER_PACKAGE.to_string()];
  if !args.is_empty() {
    cargo_args.push("--".to_string());
    cargo_args.extend_from_slice(args);
  }
  run_cargo(root, &cargo_args)?;

  if !keep {
    rollback_generated(root)?;
  }

  Ok(())
}

/// Cleans the generated artifacts, the build cache and `output/`.
///
/// Resets the generated artifacts, removes `output/` and `build_cache/`, and
/// removes `target/` unless `skip_target` is set. The project is re-validated
/// first so cleaning refuses to touch a foreign directory.
///
/// # Errors
///
/// Returns an error if the project is not a courier project, if the generated
/// files cannot be rewritten, or if a directory cannot be removed.
pub async fn clean_project(
  root: &Path, skip_target: bool,
) -> anyhow::Result<()> {
  if !checker::is_courier(root).await? {
    anyhow::bail!("Project is not courier");
  }

  if !skip_target {
    remove_dir(&target_path(root)).await?;
    info!("Cleaned `target` directory");
  }

  remove_dir(&output_path(root)).await?;
  info!("Cleaned `output` directory");

  remove_dir(&cache_path(root)).await?;
  info!("Cleaned build cache");

  register::clean_files(root)?;
  info!("Cleaned generated files");

  Ok(())
}

/// Reads the project root from the compile-time layout, then verifies it is a
/// courier project.
///
/// # Errors
///
/// Returns an error if the project root cannot be resolved or the root
/// manifest is not a courier workspace manifest.
pub async fn locate_project() -> anyhow::Result<PathBuf> {
  let root = project_root()?;

  if !checker::is_courier(&root).await? {
    anyhow::bail!("Project is not courier");
  }

  Ok(root)
}

/// Discovers every module when `with_all` is set, otherwise filters the
/// discovery down to `names` (empty `names` selects none).
///
/// # Errors
///
/// Returns an error if the `builtins` directory cannot be read or a requested
/// module name is unknown.
pub fn selected_modules(
  root: &Path, with_all: bool, names: &[String],
) -> anyhow::Result<Vec<BuiltinModule>> {
  let all = register::list_all_modules(root)?;

  if with_all { Ok(all) } else { register::select_modules(&all, names) }
}
