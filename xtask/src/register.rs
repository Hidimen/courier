//! Built-in module discovery and artifact generation.
//!
//! This module discovers the built-in modules declared in the `builtins`
//! directory of the project, filters them by the `--with-modules` selection,
//! then rewrites the artifacts that wire the selected modules into the
//! workspace:
//!
//! - `waybill/Cargo.toml` — the `# AUTOGEN DEP: START #` dependency section,
//!   registering each selected module as a path dependency;
//! - `waybill/src/builtin/modules.rs` — the generated module table holding
//!   `register_all`, which calls every selected module's
//!   `builtin_module_entry`.
//!
//! Generating Rust source instead of a custom registry format keeps the
//! artifacts inside Cargo's dependency graph: editing a module manifest
//! changes a tracked input, so `waybill` is rebuilt, the output is navigable
//! in an IDE, and diagnostics point at real lines.
//!
//! Every artifact is written directly into the real project tree. Unlike the
//! carrier build crate there is no `.courier` mirror and no hard-link
//! detaching step, because nothing is mirrored in the first place.

use std::fs;
use std::path::{Path, PathBuf};

use loglet::{error, info};

use crate::define::ModuleManifest;
use crate::git_cloner::GitCloner;
use crate::paths::*;
use crate::{BuiltinModule, Placeholder};

/// The generated manifest and module table paths.
struct GeneratedPaths {
  waybill_manifest: PathBuf,
  modules: PathBuf,
}

/// Resolves the generated artifact paths of the project.
fn generated_paths(root: &Path) -> GeneratedPaths {
  GeneratedPaths {
    waybill_manifest: waybill_manifest_path(root),
    modules: modules_source_path(root),
  }
}

/// Discovers every built-in module declared in the `builtins` directory of
/// the project.
///
/// Every directory containing a `Cargo.toml` is a builtin module. A plain
/// file is treated as a placeholder: it may point at a local `dir` or at a
/// `git` URL that is cloned into `build_cache/`.
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

  list.sort_by(|left, right| left.name.cmp(&right.name));

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
/// Writes the `waybill` manifest (rewriting the `# AUTOGEN DEP ... #` markers
/// with non-optional path dependencies) and the
/// `waybill/src/builtin/modules.rs` module table read by the `waybill`
/// composition root. The `[features]` section of the manifest is never
/// touched.
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

  let waybill_manifest = fs::read_to_string(&paths.waybill_manifest)?;

  let waybill_dep: Vec<String> = modules
    .iter()
    .map(|module| {
      format!("{} = {{ path = \"{}\" }}", module.name, dep_path(root, module))
    })
    .collect();
  let modules = render_modules(modules);

  let waybill_manifest =
    replace_with_marker("DEP", &waybill_manifest, &waybill_dep.join("\n"));

  write_generated(&paths, modules, waybill_manifest)
}

/// Resets all generated artifacts to their empty, module-free state.
///
/// Rewrites the `# AUTOGEN DEP ... #` markers with an empty block and writes
/// an empty module table.
///
/// # Errors
///
/// Returns an error if any of the target files cannot be read or written.
pub fn clean_files(root: &Path) -> anyhow::Result<()> {
  let paths = generated_paths(root);

  let waybill_manifest = fs::read_to_string(&paths.waybill_manifest)?;
  let waybill_manifest = replace_with_marker("DEP", &waybill_manifest, "");

  write_generated(&paths, render_modules(&[]), waybill_manifest)
}

/// Writes the generated manifest and module table.
fn write_generated(
  paths: &GeneratedPaths, modules: String, waybill_manifest: String,
) -> anyhow::Result<()> {
  write_generated_file(&paths.waybill_manifest, &waybill_manifest)?;
  info!("WRITE {}", paths.waybill_manifest.display());

  if let Some(parent) = paths.modules.parent() {
    fs::create_dir_all(parent)?;
  }
  write_generated_file(&paths.modules, &modules)?;
  info!("WRITE {}", paths.modules.display());

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

/// Parses a builtin module manifest.
///
/// Every directory under `builtins/` is a module; the manifest only needs a
/// `[package].name`. Logs an error and returns `None` when the manifest is
/// missing or malformed.
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

  let manifest: ModuleManifest = match toml::from_str(&content) {
    Ok(manifest) => manifest,
    Err(error) => {
      error!("Failed to decode {}: {}", toml_path.display(), error);
      return None;
    },
  };

  if manifest.package.name.is_empty() {
    error!("{} has an empty package name", subject.display());
    return None;
  }

  Some(BuiltinModule {
    name: manifest.package.name,
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

/// Renders the dependency path of a module relative to the `waybill` crate,
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

/// Renders the `waybill/src/builtin/modules.rs` table for the selection.
///
/// The output calls every selected module's entry function, e.g.
///
/// ```text
/// use depot::Depot;
/// use extension::Registry;
///
/// pub(crate) fn register_all(registry: &mut Registry<Depot>) {
///   echo::builtin_module_entry(registry);
/// }
/// ```
fn render_modules(modules: &[BuiltinModule]) -> String {
  let mut source = String::from(
    "//! Builtin module table.\n\
     //!\n\
     //! Generated by `xtask` from the directories under `builtins/`.\n\
     //! DO NOT EDIT.\n\
     \n\
     use depot::Depot;\n\
     use extension::builtin::Registry;\n\
     \n",
  );

  if modules.is_empty() {
    source.push_str(
      "/// Installs every builtin module compiled into this binary.\n\
       pub(crate) fn register_all(_registry: &mut Registry<Depot>) {}\n",
    );
    return source;
  }

  source.push_str(
    "/// Installs every builtin module compiled into this binary.\n\
     pub(crate) fn register_all(registry: &mut Registry<Depot>) {\n",
  );
  for module in modules {
    source.push_str(&format!(
      "  {}::builtin_module_entry(registry);\n",
      crate_name(module)
    ));
  }
  source.push_str("}\n");

  source
}

/// Returns the Rust crate name of a module, e.g. `my_module` for the package
/// `my-module`.
fn crate_name(module: &BuiltinModule) -> String {
  module.name.replace('-', "_")
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
      path: PathBuf::from("builtins").join(name),
    }
  }

  #[test]
  fn render_modules_calls_each_entry() {
    let source = render_modules(&[module("echo"), module("echo_udp")]);

    assert!(source.contains("use depot::Depot;"));
    assert!(source.contains("use extension::Registry;"));
    assert!(source.contains(
      "pub(crate) fn register_all(registry: &mut Registry<Depot>) {"
    ));
    assert!(source.contains("echo::builtin_module_entry(registry);"));
    assert!(source.contains("echo_udp::builtin_module_entry(registry);"));
  }

  #[test]
  fn render_modules_is_empty_without_modules() {
    let source = render_modules(&[]);

    assert!(source.contains(
      "pub(crate) fn register_all(_registry: &mut Registry<Depot>) {}"
    ));
  }

  #[test]
  fn crate_name_replaces_hyphens() {
    assert_eq!(crate_name(&module("my-module")), "my_module");
  }

  #[test]
  fn parse_manifest_accepts_a_plain_package() {
    let dir = temp_dir("plain");
    let manifest = dir.join("Cargo.toml");
    fs::write(
      &manifest,
      "[package]\nname = \"echo\"\nversion = \"0.1.0\"\n\
       description = \"Echo builtins\"\n",
    )
    .unwrap();

    let module = parse_manifest(&manifest, &dir, &manifest).unwrap();

    assert_eq!(module.name, "echo");
    assert_eq!(&module.path, &dir);

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn parse_manifest_rejects_a_manifest_without_a_package_name() {
    let dir = temp_dir("no-name");
    let manifest = dir.join("Cargo.toml");
    fs::write(&manifest, "[package]\nversion = \"0.1.0\"\n").unwrap();

    assert!(parse_manifest(&manifest, &dir, &manifest).is_none());

    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn parse_manifest_rejects_a_missing_manifest() {
    let dir = temp_dir("missing");

    assert!(parse_manifest(&dir.join("Cargo.toml"), &dir, &dir).is_none());

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
