use std::{
  io::{Error, ErrorKind, Result},
  net::SocketAddr,
  task::{Context, Poll},
};

pub trait Stream: Send + Sync + 'static {
  type ReadHalf: ReadHalf + Send + Sync + 'static;
  type WriteHalf: WriteHalf + Send + Sync + 'static;

  /// Reads data from the stream into `buf`.
  ///
  /// # Returns
  ///
  /// The number of bytes read. `0` indicates the peer has closed the
  /// read half of the stream.
  fn read(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  /// Writes data from `buf` to the stream.
  ///
  /// # Returns
  ///
  /// The number of bytes written.
  fn write(&mut self, buf: &[u8])
  -> impl Future<Output = Result<usize>> + Send;

  /// Flushes any buffered writes to the underlying transport.
  fn flush(&mut self) -> impl Future<Output = Result<()>> + Send;

  /// Peeks at data in the stream without consuming it.
  ///
  /// # Returns
  ///
  /// The number of bytes peeked. `0` indicates the peer has closed the
  /// read half.
  fn peek(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  /// Attempts a non-blocking read from the stream.
  ///
  /// Returns immediately:
  /// - `Ok(n)` where `n > 0` — data was read.
  /// - `Ok(0)` — the peer has closed the read half (EOF).
  /// - `Err(ref e)` if `e.kind() == WouldBlock` — stream is open but
  ///   no data is available right now.
  /// - `Err(e)` — a fatal error (connection reset, etc.).
  ///
  /// **Note**: this method may consume data from the stream.
  fn try_read(&self, buf: &mut [u8]) -> Result<usize>;

  /// Attempts a non-blocking write to the stream.
  ///
  /// Returns immediately:
  /// - `Ok(n)` — `n` bytes were written.
  /// - `Err(ref e)` if `e.kind() == WouldBlock` — the write would
  ///   block (stream buffer is full).
  /// - `Err(e)` — a fatal error (broken pipe, etc.).
  fn try_write(&self, buf: &[u8]) -> Result<usize>;

  /// Returns the peer address of this stream.
  fn peer_addr(&self) -> Result<SocketAddr>;

  /// Returns the local address of this stream.
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

  fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

pub trait ReadHalf: Send + Sync + Sized + 'static {
  /// Reads data from the read half into `buf`.
  ///
  /// # Returns
  ///
  /// The number of bytes read. `0` means the peer has closed.
  fn read(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  /// Returns the local address of this stream.
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

  /// Peeks at data without consuming it.
  fn peek(
    &mut self, buf: &mut [u8],
  ) -> impl Future<Output = Result<usize>> + Send;

  /// Attempts to peek at pending data without consuming it, without
  /// yielding.
  ///
  /// Unlike [`peek`](Self::peek), this polls the underlying socket
  /// once and reports readiness through [`Poll`] instead of waiting.
  ///
  /// # Returns
  ///
  /// - `Poll::Ready(Ok(n))` with `n > 0` — `n` bytes were peeked into
  ///   `buf` without being consumed.
  /// - `Poll::Ready(Ok(0))` — the peer has closed the read half (EOF).
  /// - `Poll::Ready(Err(e))` — a fatal error occurred.
  /// - `Poll::Pending` — the stream is open but no data is available
  ///   right now.
  fn poll_peek(
    &mut self, cx: &mut Context<'_>, buf: &mut [u8],
  ) -> Poll<Result<usize>>;

  /// Attempts a non-blocking read from the read half.
  ///
  /// Returns immediately:
  /// - `Ok(n)` where `n > 0` — data was read.
  /// - `Ok(0)` — the peer has closed (EOF).
  /// - `Err(ref e)` if `e.kind() == WouldBlock` — open but no data
  ///   available right now.
  /// - `Err(e)` — a fatal error.
  fn try_read(&self, buf: &mut [u8]) -> Result<usize>;
}

pub trait WriteHalf: Send + Sync + Sized + 'static {
  /// Writes data from `buf` to the write half of the stream.
  ///
  /// # Returns
  ///
  /// The number of bytes written.
  fn write(&mut self, buf: &[u8])
  -> impl Future<Output = Result<usize>> + Send;

  /// Returns the local address of this stream.
  fn local_addr(&self) -> Result<SocketAddr>;

  /// Returns the peer address of this stream.
  fn peer_addr(&self) -> Result<SocketAddr>;

  /// Attempts a non-blocking write to the write half.
  ///
  /// Returns immediately:
  /// - `Ok(n)` — `n` bytes were written.
  /// - `Err(ref e)` if `e.kind() == WouldBlock` — the write would
  ///   block.
  /// - `Err(e)` — a fatal error (broken pipe, etc.).
  fn try_write(&self, buf: &[u8]) -> Result<usize>;

  fn write_all(
    &mut self, mut buf: &[u8],
  ) -> impl Future<Output = Result<()>> + Send {
    async move {
      while !buf.is_empty() {
        let n = self.write(buf).await?;
        if n == 0 {
          return Err(Error::new(ErrorKind::WriteZero, "write returned Ok(0)"))
        }
        buf = &buf[n..];
      }

      Ok(())
    }
  }

  fn flush(&mut self) -> impl Future<Output = Result<()>> + Send;
}
