use gix::head::Kind;

fn main() -> anyhow::Result<()> {
  let repo = gix::open(env!("CARGO_MANIFEST_PATH"))?;
  let commit_id = repo.head_id()?;

  let head = repo.head()?;
  let branch_name = match head.kind {
    Kind::Symbolic(r) => r.name.shorten().to_string(),
    Kind::Detached { .. } => String::from("<Detached>"),
    Kind::Unborn(_) => String::from("new"),
  };

  println!("cargo:rustc-env=COMMIT_ID={commit_id}");
  println!("cargo:rustc-env=BRANCH={branch_name}");

  Ok(())
}
