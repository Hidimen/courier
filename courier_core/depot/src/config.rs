use logger::Level;
use marquage::{Generate, Parse};

#[derive(Debug, Generate, Parse)]
pub struct Config {
  pub server: ServerConfig,
}

#[derive(Debug, Parse, Generate)]
pub struct ServerConfig {
  pub bootloader: Option<String>,
  pub logger: Option<LoggerConfig>,
}

#[derive(Debug, Parse, Generate)]
pub struct LoggerConfig {
  #[marquage(default)]
  level: Level,
}
