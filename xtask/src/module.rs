use std::path::PathBuf;

/// A builtin module discovered in the `builtins` directory of the project.
#[derive(Clone, Debug)]
pub struct BuiltinModule {
  /// Crate name of the module, e.g. `echo`.
  pub name: String,
  /// The provider declared by `[package.metadata.extension]`, e.g.
  /// `EchoProvider as stream`. A module without a provider is registered
  /// without any provider entry.
  pub provider: Option<String>,
  /// Registry description of the module.
  pub description: String,
  /// Version of the module crate.
  pub version: String,
  /// Path of the module crate.
  pub path: PathBuf,
}
