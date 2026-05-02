use std::{
  io::{Error, ErrorKind, Result},
  net::SocketAddr,
};

use async_trait::async_trait;

#[async_trait]
pub trait Stream: Send + Sync + 'static {
  /// Read from an exist stream.
  ///
  /// # Returns
  /// `usize` means exact bytes read. 0 means the peer is closed.
  async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

  async fn write(&mut self, buf: &[u8]) -> Result<usize>;

  async fn flush(&mut self) -> Result<()>;

  async fn peek(&mut self, buf: &mut [u8]) -> Result<usize>;

  fn peer_addr(&self) -> Result<SocketAddr>;

  fn local_addr(&self) -> Result<SocketAddr>;

  async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
    let mut read_len = 0;
    while read_len < buf.len() {
      let n = self.read(&mut buf[read_len..]).await?;
      if n == 0 {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF"));
      }

      read_len += n;
    }

    Ok(())
  }

  async fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
      let n = self.write(buf).await?;
      buf = &buf[n..];
    }

    Ok(())
  }
}

#[async_trait]
pub trait ReadHalf: Send + Sync + Sized + 'static {
  async fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

  fn local_addr(&self) -> Result<SocketAddr>;

  async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
    let mut read_len = 0;
    while read_len < buf.len() {
      let n = self.read(&mut buf[read_len..]).await?;
      if n == 0 {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF"));
      }

      read_len += n;
    }

    Ok(())
  }
}

#[async_trait]
pub trait WriteHalf: Send + Sync + Sized + 'static {
  async fn write(&mut self, buf: &[u8]) -> Result<usize>;

  fn local_addr(&self) -> Result<SocketAddr>;

  fn peer_addr(&self) -> Result<SocketAddr>;

  async fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
      let n = self.write(buf).await?;
      buf = &buf[n..];
    }

    Ok(())
  }

  async fn flush(&mut self) -> Result<()>;
}

pub trait SplitStream: Stream {
  type ReadHalf: ReadHalf + Send + Sync + 'static;
  type WriteHalf: WriteHalf + Send + Sync + 'static;

  fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}
