use clap::{Parser, Subcommand};

use crate::commands::start::StartArgs;

/// Control CLI for Courier
#[derive(Parser)]
#[command(name = "courier")]
#[command(version, about, long_about = None)]
#[command(help_template = "\
{before-help}{name} {version} - {about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
")]
pub struct Cli {
  #[command(subcommand)]
  commands: Commands,
}

impl Cli {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self::parse()
  }

  pub async fn execute(self) {
    match &self.commands {
      Commands::Start(args) => args.execute().await,
    }
  }
}

#[derive(Subcommand)]
enum Commands {
  /// Start a server instance
  Start(StartArgs),
}
