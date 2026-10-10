use depot::control::standard;
use ipc::Response;

use crate::error::ResponseError;

pub enum CommandResponse {
  Ping,
}

impl Response for CommandResponse {
  type Error = ResponseError;

  fn from_framed(framed: ipc::Framed) -> Result<Self, Self::Error> {
    match framed.kind {
      standard::PING_CODE => Ok(Self::Ping),
      other => Err(ResponseError::UnknownKindCode(other)),
    }
  }

  fn handle(&mut self) -> Result<(), Self::Error> {
    match self {
      Self::Ping => println!("Pong"),
    }

    Ok(())
  }
}
