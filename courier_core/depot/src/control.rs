use bytes::Bytes;
use ipc::Command;

use crate::{Depot, error::CommandError};

pub enum DepotCommand {
  Ping,
  QueryPods,
}

impl Command for DepotCommand {
  type Error = CommandError;

  fn from_framed(framed: ipc::Framed) -> Result<Self, Self::Error> {
    match framed.kind {
      standard::PING_CODE => Ok(Self::Ping),
      standard::QUERY_POD_CODE => Ok(Self::QueryPods),
      other => Err(CommandError::UnknownKindCode(other)),
    }
  }

  async fn handle(&self) -> Result<(u16, bytes::Bytes), Self::Error> {
    match self {
      Self::Ping => Ok((standard::PING_CODE, Bytes::new())),
      Self::QueryPods => {
        let depot = Depot::get_instance();
        let list = depot.list_pods_info();
        Ok((standard::QUERY_POD_CODE, standard::from_pod_list(list)))
      },
    }
  }
}

pub mod standard {
  use bytes::{BufMut, Bytes, BytesMut};

  pub const VERSION: u8 = 1;

  pub const PING_CODE: u16 = 0;
  pub const QUERY_POD_CODE: u16 = 1;

  pub fn from_pod_list(pods: Vec<(u32, String)>) -> Bytes {
    let mut buf = BytesMut::new();
    for (id, name) in pods {
      buf.put_u32(id);
      let name_bytes = Bytes::from(name);
      buf.put_u32(name_bytes.len() as u32);
      buf.extend(name_bytes);
    }

    println!("{:?}", buf);

    buf.freeze()
  }

  fn read_u32(
    data: &[u8], pos: &mut usize,
  ) -> Result<u32, error::PodParseError> {
    let bytes =
      data.get(*pos..*pos + 4).ok_or(error::PodParseError::UnexpectedEof)?;
    *pos += 4;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
  }

  pub fn into_pod_list(
    buf: Bytes,
  ) -> Result<Vec<(u32, String)>, error::PodParseError> {
    let mut result = Vec::new();
    let mut pos = 0_usize;

    while pos < buf.len() {
      let num1 = read_u32(&buf, &mut pos)?;
      let str_len = read_u32(&buf, &mut pos)? as usize;
      let bytes = buf
        .get(pos..pos + str_len)
        .ok_or(error::PodParseError::UnexpectedEof)?;
      let s = std::str::from_utf8(bytes)
        .map_err(error::PodParseError::InvalidUtf8)?
        .to_string();
      pos += str_len;

      result.push((num1, s));
    }

    Ok(result)
  }

  pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum PodParseError {
      #[error("Unexpected EOF")]
      UnexpectedEof,
      #[error("Invalid UTF-8: {0}")]
      InvalidUtf8(#[from] std::str::Utf8Error),
    }
  }
}
