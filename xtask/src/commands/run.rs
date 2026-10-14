//! `xtask run` — generate the artifacts and run the project.

use std::path::PathBuf;

use clap::Args;

use crate::backend;

#[derive(Debug, Args)]
pub struct RunArgs {
  /// Project root to run; defaults to the current directory.
  #[arg(long)]
  pub path: Option<PathBuf>,
  /// Add the listed builtin modules; passing none adds no module.
  #[arg(long, num_args = 1.., value_name = "NAME")]
  pub with_modules: Vec<String>,
  /// Add every discovered builtin module.
  #[arg(long, conflicts_with = "with_modules")]
  pub with_all_modules: bool,
  /// Keep the generated artifacts instead of rolling them back.
  #[arg(long)]
  pub keep: bool,
  /// Arguments forwarded to the spawned `courier` executable.
  #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
  pub args: Vec<String>,
}

pub async fn handle(args: RunArgs) -> anyhow::Result<()> {
  let root = match args.path {
    Some(path) => path,
    None => backend::locate_project().await?,
  };

  if !crate::checker::is_courier(&root).await? {
    anyhow::bail!("Project is not courier");
  }

  backend::run_project(
    &root,
    args.with_all_modules,
    &args.with_modules,
    &args.args,
    args.keep,
  )
  .await
}
