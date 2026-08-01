pub mod list;
pub mod start;
pub mod version;

use clap::Subcommand;

use crate::commands::version::VersionArgs;

#[derive(Subcommand)]
pub enum Commands {
  /// Start a server instance
  Start(start::StartArgs),
  /// Get version of software
  Version(VersionArgs),
}
