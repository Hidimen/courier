use marquage::{Generate, Parse};

/// Top-level configuration for a server instance.
#[derive(Debug, Default, Parse, Generate)]
pub struct Config {
  /// The server section.
  pub server: ServerConfig,
}

/// Configuration for a single server.
#[derive(Debug, Parse, Generate)]
pub struct ServerConfig {
  /// Name of the provider which provides boot arguments.
  pub provider: Option<String>,
  /// Socket address the server binds to. Default is 0.0.0.0:8080.
  #[marquage(default = "0.0.0.0:8080")]
  pub address: String,
  /// Logger settings.
  #[marquage(default)]
  pub logger: logger::Config,
  /// Event settings.
  #[marquage(default)]
  pub event: event::Config,
}

impl Default for ServerConfig {
  fn default() -> Self {
    Self {
      provider: None,
      address: "0.0.0.0:8080".to_string(),
      logger: logger::Config::default(),
      event: Default::default(),
    }
  }
}
