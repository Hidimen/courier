use std::path::PathBuf;

/// A builtin module discovered in the `builtins` directory of the project.
#[derive(Clone, Debug)]
pub struct BuiltinModule {
  /// Crate name of the module, e.g. `echo`.
  pub name: String,
  /// Path of the module crate.
  pub path: PathBuf,
}
