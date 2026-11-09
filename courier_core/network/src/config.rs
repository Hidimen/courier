use marquage::{Generate, Parse};

#[derive(Parse, Generate, Default)]
pub struct Config {
  #[marquage(default)]
  pub stream: StreamConfig,
}

#[derive(Parse, Generate, Default)]
pub struct StreamConfig {
  #[marquage(default = 1024)]
  pub backlog: usize,
}

#[derive(Parse, Generate, Default)]
pub struct DatagramConfig {
  /// Number of workers bound to each socket. `0` means let framework decide.
  #[marquage(default = 0)]
  pub worker: usize,
}
