use std::fmt::Display;

use marquage::{Generate, Parse};

/// Top-level configuration for a server instance.
#[derive(Debug, Parse, Generate)]
pub struct Config {
  /// Name of the provider which provides boot arguments.
  #[marquage(default)]
  pub provider: Option<String>,
  /// Network addresses server listens on.
  #[marquage(default_with = default_listen)]
  pub listen: Vec<String>,
  #[marquage(default)]
  pub kind: RunningKind,
  /// Maximal processing milliseconds for shutdown.
  #[marquage(default = 10000)]
  pub shutdown_timeout: usize,
  /// Logger settings.
  #[marquage(default)]
  pub logger: logger::Config,
  /// Event settings.
  #[marquage(default)]
  pub event: event::Config,
}

#[derive(Debug, Parse, Generate, Default)]
#[marquage(rename_all = "snake_case")]
pub enum RunningKind {
  #[default]
  Stream,
  Datagram,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      provider: None,
      listen: vec!["0.0.0.0:4030".into()],
      kind: RunningKind::Stream,
      shutdown_timeout: 10000,
      logger: logger::Config::default(),
      event: Default::default(),
    }
  }
}

fn default_listen() -> Vec<String> {
  vec!["0.0.0.0:4030".into()]
}

impl Display for RunningKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Datagram => write!(f, "datagram"),
      Self::Stream => write!(f, "stream"),
    }
  }
}
