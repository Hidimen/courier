use bytes::Bytes;
use ipc::Command;

use crate::error::CommandError;

pub enum DepotCommand {
  Ping,
}

impl Command for DepotCommand {
  type Error = CommandError;

  fn from_framed(framed: ipc::Framed) -> Result<Self, Self::Error> {
    match framed.kind {
      standard::PING_CODE => Ok(Self::Ping),
      other => Err(CommandError::UnknownKindCode(other)),
    }
  }

  async fn handle(&self) -> Result<(u16, bytes::Bytes), Self::Error> {
    match self {
      Self::Ping => Ok((standard::PING_CODE, Bytes::new())),
    }
  }
}

pub mod standard {
  pub const VERSION: u8 = 1;

  pub const PING_CODE: u16 = 0;
}
