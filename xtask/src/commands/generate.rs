//! `xtask generate` — write the builtin artifacts without building.

use clap::Args;

use crate::backend;

#[derive(Debug, Args)]
pub struct GenerateArgs {
  /// Add the listed builtin modules; passing none adds no module.
  #[arg(long, num_args = 1.., value_name = "NAME")]
  pub with_modules: Vec<String>,
  /// Add every discovered builtin module.
  #[arg(long, conflicts_with = "with_modules")]
  pub with_all_modules: bool,
}

pub async fn handle(args: GenerateArgs) -> anyhow::Result<()> {
  let root = backend::locate_project().await?;

  backend::generate_selection(
    &root,
    args.with_all_modules,
    &args.with_modules,
  )?;

  Ok(())
}
