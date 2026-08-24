use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct TestArgs {
  #[command(subcommand)]
  subcommands: TestSubcommands,
}

#[derive(Debug, Subcommand)]
enum TestSubcommands {
  /// Test config parser capability.
  #[command(name = "config.parse")]
  ConfigParse(config::ParseArgs),
}

pub async fn handle(args: TestArgs) -> anyhow::Result<()> {
  match args.subcommands {
    TestSubcommands::ConfigParse(sub_args) => {
      config::handle_parse(sub_args).await
    },
  }
}

mod config {
  use std::path::PathBuf;

  use anyhow::Context;
  use clap::{Args, ValueEnum};
  use courier::depot::config::Config;
  use marquage::Map;

  #[derive(Debug, Args)]
  pub struct ParseArgs {
    /// Path to config file.
    #[arg(short, long)]
    path: PathBuf,
    /// Determine how to parse config file.
    #[arg(short, long, default_value = "plain")]
    kind: ParseKind,
  }

  #[derive(Debug, ValueEnum, Clone)]
  #[value(rename_all = "kebab-case")]
  enum ParseKind {
    ServerConfig,
    Plain,
  }

  pub async fn handle_parse(args: ParseArgs) -> anyhow::Result<()> {
    let content =
      std::fs::read(args.path).context("Cannot read config file")?;
    match args.kind {
      ParseKind::ServerConfig => {
        let config: Config =
          marquage::from_slice(&content).context("Cannot parse config file")?;
        println!("Server config file:\n{:#?}", config);
      },
      ParseKind::Plain => {
        let config: Map =
          marquage::from_slice(&content).context("Cannot parse config file")?;
        println!("Config file:\n{:#?}", config);
      },
    }
    Ok(())
  }
}
