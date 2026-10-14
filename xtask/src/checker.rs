//! Validation of the project the build tool is pointed at.

use std::path::Path;

use anyhow::Context;
use tokio::fs as async_fs;

use crate::define::CourierManifest;

/// Returns whether `path` is the root of a courier project, i.e. its root
/// manifest carries a `[workspace.metadata.extension]` section of kind
/// `courier`. The section's historical `carrier` name and an omitted `kind`
/// (defaulting to `courier`) are accepted as well.
///
/// A malformed manifest is reported as "not a courier project" rather than as
/// an error, so the caller can print a single clear message.
///
/// # Errors
///
/// Returns an error if `path/Cargo.toml` cannot be read.
pub async fn is_courier(path: &Path) -> anyhow::Result<bool> {
  let manifest_path = path.join("Cargo.toml");

  let manifest_content = async_fs::read_to_string(&manifest_path)
    .await
    .context("Cannot access to `Cargo.toml`")?;
  match toml::from_str::<CourierManifest>(&manifest_content) {
    Ok(manifest) => {
      Ok(manifest.workspace.metadata.and_then(|m| m.extension).is_some_and(
        |metadata| {
          metadata.kind.as_deref().is_none_or(|kind| kind == "courier")
        },
      ))
    },
    Err(_) => Ok(false),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Writes a manifest into a unique temporary directory, runs
  /// [`is_courier`] against its parent and cleans the directory up again.
  fn is_courier_manifest(content: &str) -> bool {
    static NEXT: std::sync::atomic::AtomicUsize =
      std::sync::atomic::AtomicUsize::new(0);

    let dir = std::env::temp_dir().join(format!(
      "xtask-checker-{}-{}",
      std::process::id(),
      NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Cargo.toml"), content).unwrap();

    let result = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(is_courier(&dir))
      .unwrap();

    std::fs::remove_dir_all(&dir).unwrap();
    result
  }

  #[test]
  fn recognizes_an_extension_project_marker() {
    assert!(is_courier_manifest(
      "[workspace]\nmembers = []\n\n[workspace.metadata.extension]\nkind = \
       \"courier\"\n"
    ));
  }

  #[test]
  fn recognizes_the_legacy_carrier_project_marker() {
    assert!(is_courier_manifest(
      "[workspace]\nmembers = []\n\n[workspace.metadata.carrier]\nkind = \
       \"courier\"\n"
    ));
  }

  #[test]
  fn recognizes_an_extension_marker_without_a_kind() {
    assert!(is_courier_manifest(
      "[workspace]\nmembers = []\n\n[workspace.metadata.extension]\n"
    ));
  }

  #[test]
  fn rejects_a_project_without_a_marker() {
    assert!(!is_courier_manifest("[workspace]\nmembers = []\n"));
  }

  #[test]
  fn rejects_an_unknown_project_kind() {
    assert!(!is_courier_manifest(
      "[workspace]\nmembers = []\n\n[workspace.metadata.extension]\nkind = \
       \"other\"\n"
    ));
  }
}
