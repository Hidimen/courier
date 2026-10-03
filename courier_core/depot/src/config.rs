use marquage::{Generate, Parse};

/// Top-level configuration for a server instance.
#[derive(Debug, Parse, Generate)]
pub struct Config {
  /// Name of the provider which provides boot arguments.
  pub provider: Option<String>,
  #[marquage(default)]
  pub pods: Vec<PodConfig>,
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

#[derive(Debug, Parse, Generate)]
pub struct PodConfig {
  // #[marquage(default)]
  // extend_from: Option<String>,
  #[marquage(default)]
  pub listen: Vec<String>,
  #[marquage(default)]
  pub provider: Option<String>,
  #[marquage(default)]
  pub name: Option<String>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      provider: None,
      pods: Vec::new(),
      shutdown_timeout: 10000,
      logger: logger::Config::default(),
      event: Default::default(),
    }
  }
}
