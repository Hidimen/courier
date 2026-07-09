use std::{
  io::{Error, ErrorKind, Result},
  net::SocketAddr,
  time::Duration,
};

use tokio::io::{Interest, Ready};

pub trait Stream: Send + Sync + 'static {
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

  /// Waits for the stream to become ready for the given [`Interest`].
  ///
  /// Returns [`Ready::READ_CLOSED`] when the peer has closed the read
  /// half of the connection.
  ///
  /// **Note**: this method may yield indefinitely if the requested
  /// readiness is not yet available. For a non-blocking probe, use
  /// [`try_read`](Self::try_read) or [`check_state`].
  fn ready(
    &self, interest: Interest,
  ) -> impl Future<Output = Result<Ready>> + Send;

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

  /// Waits for the read half to become ready for the given [`Interest`].
  ///
  /// **Note**: this method yields until ready. For a non-blocking probe,
  /// use [`try_read`](Self::try_read).
  fn ready(
    &self, interest: Interest,
  ) -> impl Future<Output = Result<Ready>> + Send;

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

  /// Waits for the write half to become ready for the given
  /// [`Interest`].
  ///
  /// **Note**: this method yields until ready. For a non-blocking probe,
  /// use [`try_write`](Self::try_write).
  fn ready(
    &self, interest: Interest,
  ) -> impl Future<Output = Result<Ready>> + Send;

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

pub enum StreamState {
  /// Completely okay.
  Open,
  /// Peer closed the stream.
  PeerClosed,
  /// Stream is half-open.
  HalfOpen,
  /// Stream is broken.
  Broken,
}

/// Checks the state of a [`Stream`] without blocking or consuming data.
///
/// Wraps [`peek`](Stream::peek) in a zero-duration
/// [`timeout`](tokio::time::timeout) so the call returns immediately
/// rather than yielding. The write direction is probed via
/// [`try_write`](Stream::try_write).
///
/// # Examples
///
/// ```rust,ignore
/// use network::stream::{check_state, StreamState};
///
/// # async fn example(stream: &mut impl network::stream::Stream) {
/// match check_state(stream).await {
///   StreamState::Open => println!("stream is healthy"),
///   StreamState::PeerClosed => {
///     println!("peer closed their end")
///   }
///   _ => println!("stream is broken"),
/// }
/// # }
/// ```
pub async fn check_state<R, W>(read: &mut R, write: &mut W) -> StreamState
where
  R: ReadHalf,
  W: WriteHalf,
{
  // Probe write direction with an empty buffer — detects broken pipe
  // without consuming send-buffer space.
  match write.try_write(&[]) {
    Err(e)
      if e.kind() == ErrorKind::BrokenPipe
        || e.kind() == ErrorKind::ConnectionReset
        || e.kind() == ErrorKind::ConnectionAborted =>
    {
      return StreamState::Broken;
    },
    _ => {},
  }

  // Peek with zero-duration timeout: if the socket is idle the
  // timeout fires immediately with `Elapsed`, meaning the stream is
  // open but has no data pending.  `peek` does not consume data.
  let mut buf = [0u8; 1];
  match tokio::time::timeout(Duration::ZERO, read.peek(&mut buf)).await {
    // Peek completed — check the result.
    Ok(Ok(0)) => StreamState::PeerClosed,
    Ok(Ok(_)) => StreamState::Open,
    Ok(Err(_)) => StreamState::Broken,
    // Timeout elapsed before peek could complete — stream is open
    // but idle (no data available yet).
    Err(_elapsed) => StreamState::Open,
  }
}
