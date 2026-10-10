use std::io::{Result, Write};

use bytes::Bytes;
use interprocess::local_socket::{GenericNamespaced, prelude::*};

use crate::{Framed, Response};

pub struct Client {
  socket: LocalSocketStream,
}

impl Client {
  pub fn new() -> Result<Self> {
    let name = constants::IPC_NAME.to_ns_name::<GenericNamespaced>()?;

    Ok(Self { socket: LocalSocketStream::connect(name)? })
  }

  pub fn send<R: Response>(
    &mut self, version: u8, kind: u16, payload: Bytes,
  ) -> anyhow::Result<R> {
    self.socket.write_all(
      &Framed { version, kind, len: payload.len() as u32, payload }
        .into_bytes(),
    )?;

    let framed = Framed::parse_sync(&mut self.socket)?;
    let cmd = R::from_framed(framed)?;

    Ok(cmd)
  }
}
