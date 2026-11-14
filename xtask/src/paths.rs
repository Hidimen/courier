//! Paths of the project tree the build tool operates on.

use std::path::{Path, PathBuf};

/// Returns the root of the courier project being built.
///
/// `xtask` is invoked from an arbitrary working directory, so the root is
/// derived from the compile-time location of this crate
/// (`<root>/xtask/src/...`) rather than from the current directory.
///
/// # Errors
///
/// Returns an error if the crate directory is not available at compile time
/// or does not have the expected `<root>/xtask` layout.
pub fn project_root() -> anyhow::Result<PathBuf> {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

  crate_dir
    .parent()
    .map(Path::to_path_buf)
    .ok_or_else(|| anyhow::anyhow!("xtask crate has no parent directory"))
}

/// Returns the `builtins` directory of the project.
pub fn builtins_path(root: &Path) -> PathBuf {
  root.join("builtins")
}

/// Returns the path of the `waybill` crate manifest.
pub fn waybill_manifest_path(root: &Path) -> PathBuf {
  root.join("waybill").join("Cargo.toml")
}

/// Returns the path of the generated module table of the `waybill` crate.
pub fn modules_source_path(root: &Path) -> PathBuf {
  root.join("waybill").join("src").join("builtin").join("modules.rs")
}

/// Returns the `target` directory of the project.
pub fn target_path(root: &Path) -> PathBuf {
  root.join("target")
}

/// Returns the `output` directory where built executables are copied.
pub fn output_path(root: &Path) -> PathBuf {
  root.join("output")
}

/// Returns the `build_cache` directory where Git placeholder modules are
/// cloned.
pub fn cache_path(root: &Path) -> PathBuf {
  root.join("build_cache")
}
