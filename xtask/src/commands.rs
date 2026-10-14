//! Subcommands exposed by the build tool.

mod build;
mod clean;
mod generate;
mod run;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
  /// Build the courier executable into `output/`.
  Build(build::BuildArgs),
  /// Run the project, forwarding the remaining arguments to it.
  Run(run::RunArgs),
  /// Clean generated artifacts, the build cache and `output/`.
  Clean(clean::CleanArgs),
  /// Generate the builtin artifacts without building.
  Generate(generate::GenerateArgs),
}

impl Commands {
  /// Dispatches to the handler of the selected subcommand.
  ///
  /// # Errors
  ///
  /// Returns any error raised by the selected subcommand.
  pub async fn execute(self) -> anyhow::Result<()> {
    match self {
      Self::Build(args) => build::handle(args).await,
      Self::Run(args) => run::handle(args).await,
      Self::Clean(args) => clean::handle(args).await,
      Self::Generate(args) => generate::handle(args).await,
    }
  }
}
