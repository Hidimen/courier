use std::{
  io::{Error, ErrorKind, Result},
  net::SocketAddr,
};

pub trait Stream: Send + Sync + 'static {
  /// Read from an exist stream.
  ///
  /// # Returns
  /// `usize` means exact bytes read. 0 means the peer is closed.
  fn read(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  fn write(&mut self, buf: &[u8])
  -> impl Future<Output = Result<usize>> + Send;

  fn flush(&mut self) -> impl Future<Output = Result<()>> + Send;

  fn peek(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  fn peer_addr(&self) -> Result<SocketAddr>;

  fn local_addr(&self) -> Result<SocketAddr>;

  fn read_exact(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<()>> + Send {
    async {
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

  fn write_all(
    &mut self, mut buf: &[u8],
  ) -> impl Future<Output = Result<()>> + Send {
    async move {
      while !buf.is_empty() {
        let n = self.write(buf).await?;
        buf = &buf[n..];
      }

      Ok(())
    }
  }
}

pub trait ReadHalf: Send + Sync + Sized + 'static {
  fn read(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  fn local_addr(&self) -> Result<SocketAddr>;

  fn read_exact(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<()>> + Send {
    async {
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
}

pub trait WriteHalf: Send + Sync + Sized + 'static {
  fn write(&mut self, buf: &[u8])
  -> impl Future<Output = Result<usize>> + Send;

  fn local_addr(&self) -> Result<SocketAddr>;

  fn peer_addr(&self) -> Result<SocketAddr>;

  fn write_all(
    &mut self, mut buf: &[u8],
  ) -> impl Future<Output = Result<()>> + Send {
    async move {
      while !buf.is_empty() {
        let n = self.write(buf).await?;
        buf = &buf[n..];
      }

      Ok(())
    }
  }

  fn flush(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub trait SplitStream: Stream {
  type ReadHalf: ReadHalf + Send + Sync + 'static;
  type WriteHalf: WriteHalf + Send + Sync + 'static;

  fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}
