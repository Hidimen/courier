//! `xtask build` — build the project into `output/`.

use std::path::PathBuf;

use clap::Args;
use loglet::info;

use crate::backend;

#[derive(Debug, Args)]
pub struct BuildArgs {
  /// Build the `release` profile instead of `debug`.
  #[arg(long)]
  pub release: bool,
  /// Project root to build; defaults to the current directory.
  #[arg(long)]
  pub path: Option<PathBuf>,
  /// Directory receiving the `courier` executable; defaults to `<root>/output`.
  #[arg(long)]
  pub output: Option<PathBuf>,
  /// Add the listed builtin modules; passing none adds no module.
  #[arg(long, num_args = 1.., value_name = "NAME")]
  pub with_modules: Vec<String>,
  /// Add every discovered builtin module.
  #[arg(long, conflicts_with = "with_modules")]
  pub with_all_modules: bool,
  /// Keep the generated artifacts instead of rolling them back.
  #[arg(long)]
  pub keep: bool,
  /// Extra arguments forwarded to `cargo build`.
  #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
  pub args: Vec<String>,
}

pub async fn handle(args: BuildArgs) -> anyhow::Result<()> {
  let root = match args.path {
    Some(path) => path,
    None => backend::locate_project().await?,
  };

  if !crate::checker::is_courier(&root).await? {
    anyhow::bail!("Project is not courier");
  }

  let output = args.output.unwrap_or_else(|| root.join("output"));
  let mode = if args.release { "release" } else { "debug" };

  backend::build_project(
    &root,
    mode,
    &output,
    args.with_all_modules,
    &args.with_modules,
    &args.args,
    args.keep,
  )
  .await?;

  info!("Built `{}` profile into {}", mode, output.display());

  Ok(())
}
