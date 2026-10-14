//! `xtask clean` — reset generated artifacts and remove build outputs.

use clap::Args;
use loglet::info;

use crate::backend;

#[derive(Debug, Args)]
pub struct CleanArgs {
  /// Keep the `target` directory so the next build stays incremental.
  #[arg(long)]
  pub skip_target: bool,
}

pub async fn handle(args: CleanArgs) -> anyhow::Result<()> {
  let root = backend::locate_project().await?;

  backend::clean_project(&root, args.skip_target).await?;
  info!("Cleaned");

  Ok(())
}
