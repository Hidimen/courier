use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, ValueEnum};
use courier::depot::config::Config;
use logger::Level;

/// Arguments for the `start` subcommand.
#[derive(Args)]
pub struct StartArgs {
  /// Path to the config file.
  #[arg(long, short)]
  config_file: Option<PathBuf>,
  /// Name of the provider to use.
  /// Overrides the `server.provider` value in the config file.
  #[arg(long, short)]
  provider: Option<String>,
  #[arg(long)]
  log_level: Option<LogLevel>,
}

#[derive(ValueEnum, Clone, Copy)]
enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
  Fatal,
}

impl From<LogLevel> for Level {
  fn from(value: LogLevel) -> Self {
    match value {
      LogLevel::Trace => Self::Trace,
      LogLevel::Debug => Self::Debug,
      LogLevel::Info => Self::Info,
      LogLevel::Warn => Self::Warn,
      LogLevel::Error => Self::Error,
      LogLevel::Fatal => Self::Fatal,
    }
  }
}

pub async fn handle(args: StartArgs) -> anyhow::Result<()> {
  let mut config = match &args.config_file {
    Some(path) => {
      let content =
        std::fs::read(path).context("Unable to read config file")?;
      marquage::from_slice(&content).context("Unable to parse config file")?
    },
    None => Config::default(),
  };

  let provider =
    args.provider.or_else(|| config.server.provider.clone()).context(
      "No provider has been specified: pass `--provider` or set \
       `server.provider` in the config file",
    )?;

  if let Some(log_level) = args.log_level {
    config.server.logger.level = log_level.into();
  }

  builtin::run_instance(&provider, &config).await
}
