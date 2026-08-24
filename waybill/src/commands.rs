mod list;
mod start;
mod test;
mod version;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
  /// Start a server instance.
  Start(start::StartArgs),
  /// Get version of software.
  Version(version::VersionArgs),
  #[cfg(debug_assertions)]
  /// Run specific test sets.
  Test(test::TestArgs),
}

impl Commands {
  pub async fn execute(self) -> anyhow::Result<()> {
    match self {
      Self::Start(args) => start::handle(args).await,
      Self::Version(args) => version::handle(args).await,
      #[cfg(debug_assertions)]
      Self::Test(args) => test::handle(args).await,
    }
  }
}
