//! Command-line interface of the build tool.

use clap::Parser;

use crate::commands::Commands;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(version, about = "Build tool for the courier project")]
#[command(help_template = "\
{before-help}{name} {version}
{usage-heading} {usage}

{all-args}{after-help}
")]
pub struct Cli {
  #[command(subcommand)]
  commands: Commands,
}

impl Cli {
  /// Parses the process arguments into a [`Cli`].
  pub fn parse() -> Self {
    <Self as Parser>::parse()
  }

  /// Executes the selected subcommand.
  ///
  /// # Errors
  ///
  /// Returns any error raised by the selected subcommand.
  pub async fn execute(self) -> anyhow::Result<()> {
    self.commands.execute().await
  }
}
