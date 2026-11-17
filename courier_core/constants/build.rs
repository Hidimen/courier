use std::{
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use gix::head::Kind;

fn main() -> anyhow::Result<()> {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

  println!("cargo:rustc-env=COMPILE_TIME={now}");

  match gix::open(
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .parent()
      .unwrap(),
  ) {
    Ok(repo) => {
      let commit_id = repo.head_id()?;

      let head = repo.head()?;
      let branch_name = match head.kind {
        Kind::Symbolic(r) => r.name.shorten().to_string(),
        Kind::Detached { .. } => String::from("<Detached>"),
        Kind::Unborn(_) => String::from("new"),
      };

      println!("cargo:rustc-env=COMMIT_ID={commit_id}");
      println!("cargo:rustc-env=BRANCH={branch_name}");
    },
    Err(_) => {
      println!("cargo:rustc-env=COMMIT_ID=<UNKNOWN>");
      println!("cargo:rustc-env=BRANCH=<UNKNOWN>");
    },
  }

  Ok(())
}
