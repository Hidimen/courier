mod list;
mod ping;
mod start;
#[cfg(debug_assertions)]
mod test;
mod version;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
  /// Start a server instance.
  #[command(alias = "run")]
  Start(start::StartArgs),
  /// Get version of software.
  #[command(alias = "ver")]
  Version(version::VersionArgs),
  #[cfg(debug_assertions)]
  /// Run specific test sets.
  Test(test::TestArgs),
  /// Ping the running instance.
  Ping(ping::PingArgs),
  /// List current pods.
  List(list::ListArgs),
}

impl Commands {
  pub fn execute(self) -> anyhow::Result<()> {
    match self {
      Self::Start(args) => start::handle(args),
      Self::Version(args) => version::handle(args),
      #[cfg(debug_assertions)]
      Self::Test(args) => test::handle(args),
      Self::Ping(args) => ping::handle(args),
      Self::List(args) => list::handle(args),
    }
  }
}
