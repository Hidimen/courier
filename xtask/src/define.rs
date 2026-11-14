use serde::Deserialize;

/// A courier project root manifest, used to verify the target project.
#[derive(Deserialize)]
pub struct CourierManifest {
  pub workspace: Workspace,
}

#[derive(Deserialize)]
pub struct Workspace {
  pub metadata: Option<WorkspaceMetadata>,
}

#[derive(Deserialize)]
pub struct WorkspaceMetadata {
  /// The `[workspace.metadata.extension]` section, accepted under its
  /// historical `carrier` name as well.
  #[serde(default, alias = "carrier")]
  pub extension: Option<WorkspaceExtensionMetadata>,
}

/// The `[workspace.metadata.extension]` section of a courier project root.
#[derive(Deserialize)]
pub struct WorkspaceExtensionMetadata {
  /// Project kind, e.g. `courier`. Absent means the default: `courier`.
  #[serde(default)]
  pub kind: Option<String>,
}

/// A builtin module crate manifest (`builtins/<name>/Cargo.toml`).
#[derive(Deserialize)]
pub struct ModuleManifest {
  pub package: ModulePackage,
}

#[derive(Deserialize)]
pub struct ModulePackage {
  /// Crate name of the module, e.g. `echo`.
  pub name: String,
}
