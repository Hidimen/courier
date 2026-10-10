use std::io::{Read, Result};

use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::AsyncReadExt;

pub struct Framed {
  pub version: u8,
  pub kind: u16,
  pub len: u32,
  pub payload: Bytes,
}

impl Framed {
  pub async fn parse<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Self> {
    let mut header: [u8; 7] = [0; 7];

    reader.read_exact(&mut header).await?;

    let version = u8::from_be_bytes([header[0]]);
    let kind = u16::from_be_bytes([header[1], header[2]]);
    let len = u32::from_be_bytes([header[3], header[4], header[5], header[6]]);

    let mut buf = BytesMut::with_capacity(len as usize);

    reader.read_exact(&mut buf).await?;

    Ok(Self { version, kind, len, payload: buf.freeze() })
  }

  pub fn parse_sync<R: Read>(reader: &mut R) -> Result<Self> {
    let mut header: [u8; 7] = [0; 7];

    reader.read_exact(&mut header)?;

    let version = u8::from_be_bytes([header[0]]);
    let kind = u16::from_be_bytes([header[1], header[2]]);
    let len = u32::from_be_bytes([header[3], header[4], header[5], header[6]]);

    let mut buf = BytesMut::with_capacity(len as usize);

    reader.read_exact(&mut buf)?;

    Ok(Self { version, kind, len, payload: buf.freeze() })
  }

  pub fn into_bytes(self) -> Bytes {
    let mut buf = BytesMut::with_capacity(7 + self.payload.len());

    buf.put_u8(self.version);
    buf.put_u16(self.kind);
    buf.put_u32(self.payload.len() as u32);
    buf.put(self.payload);

    buf.freeze()
  }
}
