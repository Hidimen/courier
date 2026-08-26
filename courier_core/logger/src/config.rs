use marquage::{Generate, Parse};

use crate::Level;

#[derive(Generate, Parse, Debug, Default)]
pub struct Config {
  #[marquage(default)]
  pub level: Level,
  #[marquage(default = 1024)]
  pub capacity: usize,
  #[marquage(default)]
  pub format: FormatConfig,
}

#[derive(Debug, Generate, Parse, Default)]
#[marquage(rename = "lowercase")]
pub enum FormatConfig {
  #[default]
  Default,
  Pretty,
}
