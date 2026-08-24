use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Args;
use courier::depot::config::Config;

#[derive(Args)]
pub struct StartArgs {
  /// Path to config file.
  #[arg(long, short)]
  config_file: Option<PathBuf>,
  /// Name of the bootloader.
  #[arg(long, short)]
  bootloader: Option<String>,
}

pub async fn handle(args: StartArgs) -> anyhow::Result<()> {
  // match (&args.config_file, &args.bootloader) {
  //   (_, Some(bootloader)) => builtin::run_bootloader(bootloader).await,
  //   (Some(config_file), None) => {
  //     let config_content =
  //       std::fs::read(config_file).context("Unable to read config file")?;
  //     let config: Config = marquage::from_slice(&config_content)
  //       .context("Unable to parse config file")?;
  //     if let Some(bootloader) = config.server.bootloader {
  //       builtin::run_bootloader(bootloader).await
  //     } else {
  //       bail!("No bootloader has not been specified");
  //     }
  //   },
  //   (None, None) => {
  //     bail!("No bootloader has been specified");
  //   },
  // }
  Ok(())
}
