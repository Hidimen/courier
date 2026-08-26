use marquage::{Generate, Parse};

#[derive(Debug, Parse, Generate)]
pub struct Config {
  #[marquage(default = 1024)]
  pub capacity: usize,
}

impl Default for Config {
  fn default() -> Self {
    Self { capacity: 1024 }
  }
}
