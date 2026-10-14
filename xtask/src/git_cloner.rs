//! Git placeholder cloning.
//!
//! Port of the standalone `courier-build` `git_cloner.rs` (former
//! `git_cloner.py`). It shells out to the `git` command instead of using a Git
//! library, matching how the rest of the build tooling shells out to `cargo`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Clones a Git repository into a local cache directory.
pub struct GitCloner {
  /// Directory into which the repository is cloned.
  pub path: PathBuf,
  /// Clone URL of the repository.
  pub url: String,
}

impl GitCloner {
  /// Clones the repository and returns the path of the cloned copy.
  ///
  /// The target directory is derived from the URL via [`project_name`]. Any
  /// existing copy in the cache is removed before cloning.
  ///
  /// # Errors
  ///
  /// Returns an error if the existing copy cannot be removed, if `git` cannot
  /// be spawned, or if the clone command exits with a non-zero status.
  pub fn clone_repo(&self) -> anyhow::Result<PathBuf> {
    let target_path = self.path.join(project_name(&self.url));

    if target_path.exists() {
      fs::remove_dir_all(&target_path)?;
    }

    let status = Command::new("git")
      .arg("clone")
      .arg(&self.url)
      .arg(&target_path)
      .status()?;
    if !status.success() {
      anyhow::bail!("failed to clone {}", self.url);
    }

    Ok(target_path)
  }
}

/// Extracts the project name from a clone URL.
///
/// The project name is the last path segment, without a trailing `.git`
/// suffix. Returns an empty string when the URL ends with a slash.
fn project_name(url: &str) -> &str {
  url.rsplit('/').next().unwrap_or_default().trim_end_matches(".git")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn project_name_strips_suffix_and_protocol() {
    assert_eq!(project_name("https://github.com/example/repo.git"), "repo");
    assert_eq!(project_name("https://github.com/example/repo"), "repo");
    assert_eq!(project_name("git@github.com:example/repo.git"), "repo");
    assert_eq!(project_name("https://github.com/example/"), "");
  }
}
