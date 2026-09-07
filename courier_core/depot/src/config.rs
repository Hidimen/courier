use marquage::{Generate, Parse};

/// Top-level configuration for a server instance.
#[derive(Debug, Parse, Generate)]
pub struct Config {
  /// Name of the provider which provides boot arguments.
  pub provider: Option<String>,
  /// Socket address the server binds to. Default is 0.0.0.0:8080.
  #[marquage(default = "0.0.0.0:8080")]
  pub address: String,
  /// Maximal processing milliseconds for shutdown.
  #[marquage(default = 1000)]
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
      address: "0.0.0.0:8080".to_string(),
      shutdown_timeout: 10000,
      logger: logger::Config::default(),
      event: Default::default(),
    }
  }
}
