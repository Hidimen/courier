use std::path::Path;
use std::process::Command;

fn main() {
  let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let builtins_dir = manifest_dir.parent().unwrap().join("builtins");
  let script = manifest_dir.parent().unwrap().join("build").join("build_tool.py");

  println!("cargo:rerun-if-changed={}", builtins_dir.display());
  println!("cargo:rerun-if-changed={}", script.display());

  let status = Command::new("py")
    .arg(&script)
    .status()
    .expect("failed to run build/build_tool.py -- is Python installed?");

  if !status.success() {
    panic!("build/build_tool.py failed with exit code {}", status);
  }
}
