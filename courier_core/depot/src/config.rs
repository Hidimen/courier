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

impl Default for Config {
  fn default() -> Self {
    Self {
      provider: None,
      listen: vec!["0.0.0.0:4030".into()],
      shutdown_timeout: 10000,
      logger: logger::Config::default(),
      event: Default::default(),
    }
  }
}

fn default_listen() -> Vec<String> {
  vec!["0.0.0.0:4030".into()]
}
