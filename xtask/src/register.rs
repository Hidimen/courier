//! Built-in module discovery and artifact generation.
//!
//! This module carries over the standalone `courier-build` `generate.rs`
//! (former `generate.py`): it discovers the built-in modules declared in the
//! `builtins` directory of the project, filters them by the `--with-modules`
//! selection, then rewrites the artifacts that wire the selected modules into
//! the workspace:
//!
//! - `builtin/Cargo.toml` — the `# AUTOGEN DEP: START #` dependency
//!   section, registering each selected module as a path dependency. The
//!   `[features]` section is never touched;
//! - `builtin/assets/registry.txt` — the `name: { version, description,
//!   provider }` module registry consumed by the `module_register!` proc
//!   macro in the `builtin` crate.
//!
//! Every artifact is written directly into the real project tree. Unlike the
//! carrier build crate there is no `.courier` mirror and no hard-link
//! detaching step, because nothing is mirrored in the first place.

use std::fs;
use std::path::{Path, PathBuf};

use loglet::{error, info};

use crate::define::ExtensionManifest;
use crate::git_cloner::GitCloner;
use crate::paths::*;
use crate::{BuiltinModule, Placeholder};

/// The generated manifest and registry paths.
struct GeneratedPaths {
  builtin_manifest: PathBuf,
  registry: PathBuf,
}

/// Resolves the generated artifact paths of the project.
fn generated_paths(root: &Path) -> GeneratedPaths {
  GeneratedPaths {
    builtin_manifest: builtin_manifest_path(root),
    registry: registry_asset_path(root),
  }
}

/// Discovers every built-in module declared in the `builtins` directory of
/// the project.
///
/// Each directory is expected to contain a `Cargo.toml` with a
/// `[package.metadata.extension]` section; the section's `provider` field is
/// optional. A plain file is treated as a placeholder: it may point at a
/// local `dir` or at a `git` URL that is cloned into `build_cache/`.
///
/// # Errors
///
/// Returns an error if the `builtins` directory cannot be read.
pub fn list_all_modules(root: &Path) -> anyhow::Result<Vec<BuiltinModule>> {
  let builtins = builtins_path(root);

  let mut list = Vec::new();

  for entry in fs::read_dir(builtins)? {
    let entry = entry?;
    let path = entry.path();

    if entry.file_type()?.is_dir() {
      if let Some(module) =
        parse_manifest(&path.join("Cargo.toml"), &path, &path)
      {
        list.push(module);
      }
    } else if let Some(module) = read_placeholder(root, &path)? {
      list.push(module);
    }
  }

  Ok(list)
}

/// Filters the discovered modules down to the `--with-modules` selection.
///
/// The filter starts from an empty set: passing no names selects **no** module
/// at all, matching the flag's meaning of "add these modules". Callers that
/// want every module skip this function and use [`list_all_modules`]
/// directly, which is what `--with-all-modules` does.
///
/// Unknown names are an error listing the available modules. Duplicate names
/// are ignored, and the result follows the order of `names`.
///
/// # Errors
///
/// Returns an error when any of `names` does not match a discovered module.
pub fn select_modules(
  list: &[BuiltinModule], names: &[String],
) -> anyhow::Result<Vec<BuiltinModule>> {
  if names.is_empty() {
    return Ok(vec![]);
  }

  let available: Vec<&str> =
    list.iter().map(|module| module.name.as_str()).collect();
  let unknown: Vec<&str> = names
    .iter()
    .filter(|name| !available.contains(&name.as_str()))
    .map(String::as_str)
    .collect();
  if !unknown.is_empty() {
    anyhow::bail!(
      "unknown module(s): {}; available modules: {}",
      unknown.join(", "),
      available.join(", ")
    );
  }

  let mut selected = Vec::new();
  for name in names {
    if !selected.iter().any(|module: &BuiltinModule| &module.name == name)
      && let Some(module) = list.iter().find(|module| &module.name == name)
    {
      selected.push(module.clone());
    }
  }

  Ok(selected)
}

/// Generates the files that wire the given modules into the workspace.
///
/// Writes the `builtin` manifest (rewriting the `# AUTOGEN DEP ... #` markers
/// with non-optional path dependencies) and the
/// `builtin/assets/registry.txt` module registry read by the
/// `module_register!` proc macro. The `[features]` section of the manifest is
/// never touched.
///
/// Passing an empty `modules` slice is a no-op: use [`clean_files`] to reset
/// the artifacts to their empty state.
///
/// # Errors
///
/// Returns an error if any of the target files cannot be read or written.
pub fn generate_files(
  root: &Path, modules: &[BuiltinModule],
) -> anyhow::Result<()> {
  if modules.is_empty() {
    return Ok(());
  }

  let paths = generated_paths(root);

  let builtin_manifest = fs::read_to_string(&paths.builtin_manifest)?;

  let builtin_dep: Vec<String> = modules
    .iter()
    .map(|module| {
      format!("{} = {{ path = \"{}\" }}", module.name, dep_path(root, module))
    })
    .collect();
  let registry: Vec<String> = modules.iter().map(registry_entry).collect();

  let builtin_manifest =
    replace_with_marker("DEP", &builtin_manifest, &builtin_dep.join("\n"));

  write_generated(&paths, registry.join("\n"), builtin_manifest)
}

/// Resets all generated artifacts to their empty, module-free state.
///
/// Rewrites the `# AUTOGEN DEP ... #` markers with an empty block and writes
/// the empty module registry.
///
/// # Errors
///
/// Returns an error if any of the target files cannot be read or written.
pub fn clean_files(root: &Path) -> anyhow::Result<()> {
  let paths = generated_paths(root);

  let builtin_manifest = fs::read_to_string(&paths.builtin_manifest)?;
  let builtin_manifest = replace_with_marker("DEP", &builtin_manifest, "");

  write_generated(&paths, String::new(), builtin_manifest)
}

/// Writes the generated manifest and registry asset.
fn write_generated(
  paths: &GeneratedPaths, registry: String, builtin_manifest: String,
) -> anyhow::Result<()> {
  write_generated_file(&paths.builtin_manifest, &builtin_manifest)?;
  info!("WRITE {}", paths.builtin_manifest.display());

  let assets = paths.builtin_manifest.parent().unwrap().join("assets");
  fs::create_dir_all(&assets)?;
  write_generated_file(&paths.registry, &registry)?;
  info!("WRITE {}", paths.registry.display());

  Ok(())
}

/// Writes a generated file, replacing whatever was there before.
fn write_generated_file(path: &Path, content: &str) -> anyhow::Result<()> {
  if fs::exists(path)? {
    fs::remove_file(path)?;
  }
  fs::write(path, content)?;
  Ok(())
}

/// Parses a manifest that declares an extension-managed built-in.
///
/// Logs an error and returns `None` when the manifest is missing, malformed,
/// or does not carry a `[package.metadata.extension]` section. The section
/// itself may be empty: a module without a `provider` is discovered like any
/// other and simply carries no provider entries.
fn parse_manifest(
  toml_path: &Path, crate_path: &Path, subject: &Path,
) -> Option<BuiltinModule> {
  let content = match fs::read_to_string(toml_path) {
    Ok(content) => content,
    Err(error) => {
      error!("Failed to read {}: {}", toml_path.display(), error);
      return None;
    },
  };

  let manifest: ExtensionManifest = match toml::from_str(&content) {
    Ok(manifest) => manifest,
    Err(error) => {
      error!("Failed to decode {}: {}", toml_path.display(), error);
      return None;
    },
  };

  let Some(extension) = manifest.package.metadata.and_then(|m| m.extension)
  else {
    error!(
      "{} has no `[package.metadata.extension]` section",
      subject.display()
    );
    return None;
  };

  Some(BuiltinModule {
    name: manifest.package.name,
    provider: extension.provider,
    description: extension
      .description
      .unwrap_or(manifest.package.description.unwrap_or_default()),
    version: manifest.package.version.unwrap_or_else(|| "0.0.0".to_string()),
    path: crate_path.to_path_buf(),
  })
}

/// Reads a placeholder file and resolves the built-in it points at.
///
/// A placeholder declares either a local `dir` or a `git` URL. In the latter
/// case the repository is cloned into `build_cache/` first. Returns `None`
/// when the file does not point at a resolvable built-in.
fn read_placeholder(
  root: &Path, file: &Path,
) -> anyhow::Result<Option<BuiltinModule>> {
  let content = match fs::read_to_string(file) {
    Ok(content) => content,
    Err(error) => {
      error!("Failed to read placeholder {}: {}", file.display(), error);
      return Ok(None);
    },
  };

  let placeholder: Placeholder = match toml::from_str(&content) {
    Ok(placeholder) => placeholder,
    Err(error) => {
      error!("Failed to decode placeholder {}: {}", file.display(), error);
      return Ok(None);
    },
  };

  if let Some(dir) = placeholder.dir {
    let dir = PathBuf::from(dir);
    return Ok(parse_manifest(&dir.join("Cargo.toml"), &dir, file));
  }

  if let Some(url) = placeholder.git {
    let cache_dir = cache_path(root);
    fs::create_dir_all(&cache_dir)?;

    let cloner = GitCloner { path: cache_dir, url };
    let cloned = cloner.clone_repo()?;
    return Ok(parse_manifest(&cloned.join("Cargo.toml"), &cloned, file));
  }

  Ok(None)
}

/// Replaces the contents between an `# AUTOGEN <marker>` start and end marker
/// with a generated block. Logs an error and returns the content unchanged if
/// either marker is missing.
fn replace_with_marker(marker: &str, content: &str, new: &str) -> String {
  let start_marker = format!("# AUTOGEN {marker}: START #");
  let end_marker = format!("# AUTOGEN {marker}: END #");

  let Some(start_pos) = content.find(&start_marker) else {
    error!("Could not find start marker for {marker}");
    return content.to_string();
  };

  let Some(end_pos) =
    content[start_pos + start_marker.len()..].find(&end_marker)
  else {
    error!("Could not find end marker for {marker}");
    return content.to_string();
  };
  let end_pos = start_pos + start_marker.len() + end_pos;

  let replacement = format!(
    "# AUTOGEN {marker}: START #\n# --- Generated by build script - DO NOT EDIT--- #\n{new}\n# AUTOGEN {marker}: END #"
  );

  let mut out = String::with_capacity(content.len() + replacement.len());
  out.push_str(&content[..start_pos]);
  out.push_str(&replacement);
  out.push_str(&content[end_pos + end_marker.len()..]);
  out
}

/// Renders the dependency path of a module relative to the `builtin` crate,
/// e.g. `../builtins/echo`.
///
/// Modules that live outside the project's `builtins` directory (e.g. cloned
/// placeholders) fall back to their absolute POSIX-style path.
fn dep_path(root: &Path, module: &BuiltinModule) -> String {
  let builtins_dir = builtins_path(root);

  if module.path.starts_with(&builtins_dir) {
    let folder = module.path.file_name().unwrap_or_default();
    format!("../builtins/{}", folder.to_string_lossy())
  } else {
    posix(&module.path)
  }
}

/// Renders one module registry entry, e.g.
///
/// ```text
/// echo: {
///   version => "0.1.0",
///   description => "Echo protocol",
///   provider => echo::EchoProvider as stream,
/// };
/// ```
///
/// The `provider` line is omitted for modules that declare no provider.
fn registry_entry(module: &BuiltinModule) -> String {
  let mut entry = format!(
    "{}: {{\n  version => \"{}\",\n  description => \"{}\"",
    module.name, module.version, module.description
  );

  if let Some(provider) = provider_path(module) {
    entry.push_str(&format!(",\n  provider => {provider}"));
  }

  entry.push_str(",\n};");
  entry
}

/// Renders the full provider path, prefixing the crate name unless the
/// metadata already carries it, e.g. `echo::EchoProvider as stream`.
///
/// Returns `None` for a module that declares no provider.
fn provider_path(module: &BuiltinModule) -> Option<String> {
  let provider = module.provider.as_ref()?;

  let prefix = format!("{}::", module.name);
  if provider.starts_with(&prefix) {
    Some(provider.clone())
  } else {
    Some(format!("{prefix}{provider}"))
  }
}

/// Converts a path to its POSIX-style string representation.
fn posix(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(serde::Deserialize)]
  struct Registry {
    #[serde(default)]
    features: Option<toml::Value>,
  }

  const MANIFEST: &str = r#"
[dependencies]
anyhow.workspace = true

# AUTOGEN DEP: START #
# --- Generated by build script - DO NOT EDIT--- #

# AUTOGEN DEP: END #

[features]

[dev-dependencies]
tokio.workspace = true
"#;

  /// Creates a unique empty directory for a manifest fixture.
  fn temp_dir(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize =
      std::sync::atomic::AtomicUsize::new(0);

    let dir = std::env::temp_dir().join(format!(
      "xtask-register-{tag}-{}-{}",
      std::process::id(),
      NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if fs::exists(&dir).unwrap() {
      fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn module(name: &str) -> BuiltinModule {
    BuiltinModule {
      name: name.to_string(),
      provider: Some("EchoProvider as stream".to_string()),
      description: "Echo protocol".to_string(),
      version: "0.1.0".to_string(),
      path: PathBuf::from("builtins").join(name),
    }
  }

  #[test]
  fn registry_entry_renders_version_description_and_provider() {
    let expected = "echo: {\n  version => \"0.1.0\",\n  description => \"Echo protocol\",\n  provider => echo::EchoProvider as stream,\n};";
    assert_eq!(registry_entry(&module("echo")), expected);
  }

  #[test]
  fn registry_entry_omits_the_provider_line_without_a_provider() {
    let mut module = module("http");
    module.provider = None;

    assert_eq!(
      registry_entry(&module),
      "http: {\n  version => \"0.1.0\",\n  description => \"Echo protocol\",\n};"
    );
  }

  #[test]
  fn provider_path_prefixes_the_crate_name() {
    assert_eq!(
      provider_path(&module("echo")).as_deref(),
      Some("echo::EchoProvider as stream")
    );
  }

  #[test]
  fn provider_path_keeps_an_existing_crate_prefix() {
    let mut module = module("echo");
    module.provider = Some("echo::EchoProvider as datagram".to_string());
    assert_eq!(
      provider_path(&module).as_deref(),
      Some("echo::EchoProvider as datagram")
    );
  }

  #[test]
  fn provider_path_is_none_without_a_provider() {
    let mut module = module("http");
    module.provider = None;

    assert_eq!(provider_path(&module), None);
  }

  #[test]
  fn parse_manifest_accepts_an_extension_section_without_provider() {
    let dir = temp_dir("no-provider");
    let manifest = dir.join("Cargo.toml");
    fs::write(
      &manifest,
      "[package]\nname = \"http\"\nversion = \"0.1.0\"\n\
       description = \"HTTP builtin\"\n\n[package.metadata.extension]\n",
    )
    .unwrap();

    let module = parse_manifest(&manifest, &dir, &manifest).unwrap();

    assert_eq!(module.name, "http");
    assert_eq!(module.provider, None);
    assert_eq!(module.description, "HTTP builtin");
    assert_eq!(module.version, "0.1.0");

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn parse_manifest_reads_the_provider_from_an_extension_section() {
    let dir = temp_dir("with-provider");
    let manifest = dir.join("Cargo.toml");
    fs::write(
      &manifest,
      "[package]\nname = \"echo_udp\"\nversion = \"0.1.0\"\n\n\
       [package.metadata.extension]\nprovider = \"EchoProvider as datagram\"\n",
    )
    .unwrap();

    let module = parse_manifest(&manifest, &dir, &manifest).unwrap();

    assert_eq!(module.provider.as_deref(), Some("EchoProvider as datagram"));

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn parse_manifest_accepts_the_legacy_carrier_section() {
    let dir = temp_dir("legacy-carrier");
    let manifest = dir.join("Cargo.toml");
    fs::write(
      &manifest,
      "[package]\nname = \"echo\"\nversion = \"0.1.0\"\n\n\
       [package.metadata.carrier]\nprovider = \"EchoProvider as stream\"\n",
    )
    .unwrap();

    let module = parse_manifest(&manifest, &dir, &manifest).unwrap();

    assert_eq!(module.provider.as_deref(), Some("EchoProvider as stream"));

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn parse_manifest_rejects_a_crate_without_extension_metadata() {
    let dir = temp_dir("no-metadata");
    let manifest = dir.join("Cargo.toml");
    fs::write(&manifest, "[package]\nname = \"plain\"\nversion = \"0.1.0\"\n")
      .unwrap();

    assert!(parse_manifest(&manifest, &dir, &manifest).is_none());

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn select_modules_returns_nothing_when_names_are_empty() {
    let list = vec![module("echo"), module("web")];
    assert!(select_modules(&list, &[]).unwrap().is_empty());
  }

  #[test]
  fn select_modules_filters_and_dedupes() {
    let list = vec![module("echo"), module("web")];
    let selected =
      select_modules(&list, &["web".into(), "web".into()]).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "web");
  }

  #[test]
  fn select_modules_rejects_unknown_names() {
    let list = vec![module("echo")];
    let error = select_modules(&list, &["foo".into()]).unwrap_err();
    assert!(error.to_string().contains("unknown module(s): foo"));
    assert!(error.to_string().contains("echo"));
  }

  #[test]
  fn posix_converts_backslashes() {
    assert_eq!(posix(Path::new("a\\b\\c")), "a/b/c");
    assert_eq!(posix(Path::new("a/b/c")), "a/b/c");
  }

  #[test]
  fn dep_path_is_relative_for_builtins_and_absolute_otherwise() {
    let root = Path::new("/proj");
    let mut module = module("echo");
    module.path = PathBuf::from("/proj/builtins/echo");
    assert_eq!(dep_path(root, &module), "../builtins/echo");

    module.path = PathBuf::from("C:\\cache\\echo");
    assert_eq!(dep_path(root, &module), "C:/cache/echo");
  }

  #[test]
  fn replace_with_marker_rewrites_the_block() {
    let content = "\n# AUTOGEN DEP: START #\nold\n# AUTOGEN DEP: END #\n";
    let expected = "\n# AUTOGEN DEP: START #\n# --- Generated by build script - DO NOT EDIT--- #\nnew\n# AUTOGEN DEP: END #\n";
    assert_eq!(replace_with_marker("DEP", content, "new"), expected);
  }

  #[test]
  fn replace_with_marker_returns_content_when_marker_missing() {
    let content = "no markers here";
    assert_eq!(replace_with_marker("DEP", content, "new"), content);
  }

  #[test]
  fn generated_manifest_keeps_other_sections_intact() {
    let generated = replace_with_marker(
      "DEP",
      MANIFEST,
      "echo = { path = \"../builtins/echo\" }",
    );
    let manifest: Registry = toml::from_str(&generated).unwrap();

    assert!(manifest.features.is_some());
    assert!(generated.contains("[dev-dependencies]"));
    assert!(generated.contains("anyhow.workspace = true"));
    assert!(generated.contains("echo = { path = \"../builtins/echo\" }"));
  }

  #[test]
  fn clean_files_keeps_the_manifest_parseable() {
    let cleared = replace_with_marker("DEP", MANIFEST, "");
    assert!(toml::from_str::<crate::define::Workspace>(&cleared).is_ok());
  }
}
