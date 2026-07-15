use clap::{Args, ValueEnum};

#[derive(Args)]
pub struct StartArgs {
  #[arg(long, short = 'b')]
  bootloader: BootloaderList,
}

#[derive(ValueEnum, Clone)]
enum BootloaderList {
  #[cfg(feature = "builtin_echo")]
  Echo,
}

impl StartArgs {
  pub async fn execute(&self) {
    match self.bootloader {
      #[cfg(feature = "builtin_echo")]
      BootloaderList::Echo => builtin_echo::boot().await,
    }
  }
}
