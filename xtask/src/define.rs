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

/// An extension-managed builtin crate manifest (`builtins/<name>/Cargo.toml`).
#[derive(Deserialize)]
pub struct ExtensionManifest {
  pub package: ExtensionPackage,
}

#[derive(Deserialize)]
pub struct ExtensionPackage {
  pub name: String,
  pub version: Option<String>,
  pub description: Option<String>,
  pub metadata: Option<PackageMetadata>,
}

#[derive(Deserialize)]
pub struct PackageMetadata {
  /// The `[package.metadata.extension]` section, accepted under its
  /// historical `carrier` name as well.
  #[serde(default, alias = "carrier")]
  pub extension: Option<ModuleExtensionMetadata>,
}

/// The `[package.metadata.extension]` section of a builtin module manifest.
///
/// Every field is optional: a section without fields is a valid, module-only
/// declaration that carries no provider.
#[derive(Deserialize)]
pub struct ModuleExtensionMetadata {
  /// The provider type and its transport kind, e.g.
  /// `"EchoProvider as stream"`. The crate name is prefixed when the
  /// registry is generated. Without a provider the module is registered
  /// without provider entries.
  #[serde(default)]
  pub provider: Option<String>,
  /// Optional registry description of the module.
  pub description: Option<String>,
}
