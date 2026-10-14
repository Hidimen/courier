use serde::Deserialize;

/// A placeholder file that points at an external or relocated built-in.
#[derive(Deserialize)]
pub struct Placeholder {
  /// Local directory of the built-in crate.
  pub dir: Option<String>,
  /// Git URL of the built-in crate, cloned into the build cache.
  pub git: Option<String>,
}
