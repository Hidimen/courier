use std::{
  io::Result,
  net::{SocketAddr, TcpListener, ToSocketAddrs},
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
  stream::{ReadHalf, SplitStream, Stream, WriteHalf},
  transport::{StreamTransport, Transport},
};

pub struct TcpTransport(tokio::net::TcpListener);

impl Transport for TcpTransport {
  fn bind<A: ToSocketAddrs + Send>(addr: A) -> Result<Self> {
    Ok(Self(tokio::net::TcpListener::from_std(TcpListener::bind(addr)?)?))
  }

  fn set_ttl(&self, ttl: u32) -> Result<()> {
    self.0.set_ttl(ttl)
  }

  fn ttl(&self) -> Result<u32> {
    self.0.ttl()
  }

  fn local_addr(&self) -> Result<SocketAddr> {
    self.0.local_addr()
  }
}

impl StreamTransport for TcpTransport {
  type ReadHalf = TcpReadHalf;
  type WriteHalf = TcpWriteHalf;

  type Stream = TcpStream;

  async fn accept(&self) -> Result<Self::Stream> {
    let (stream, _) = self.0.accept().await?;

    Ok(TcpStream::new(stream))
  }
}

pub struct TcpStream(tokio::net::TcpStream);

impl TcpStream {
  pub fn new(inner: tokio::net::TcpStream) -> Self {
    Self(inner)
  }
}

impl Stream for TcpStream {
  async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
    self.0.read(buf).await
  }

  async fn write(&mut self, buf: &[u8]) -> Result<usize> {
    self.0.write(buf).await
  }

  async fn flush(&mut self) -> Result<()> {
    self.0.flush().await
  }

  async fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
    self.0.peek(buf).await
  }

  async fn ready(
    &self, interest: tokio::io::Interest,
  ) -> Result<tokio::io::Ready> {
    self.0.ready(interest).await
  }

  fn try_read(&self, buf: &mut [u8]) -> Result<usize> {
    self.0.try_read(buf)
  }

  fn try_write(&self, buf: &[u8]) -> Result<usize> {
    self.0.try_write(buf)
  }

  fn peer_addr(&self) -> Result<SocketAddr> {
    self.0.peer_addr()
  }

  fn local_addr(&self) -> Result<SocketAddr> {
    self.0.local_addr()
  }
}

impl SplitStream for TcpStream {
  type ReadHalf = TcpReadHalf;
  type WriteHalf = TcpWriteHalf;

  fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
    let (read, write) = self.0.into_split();

    (TcpReadHalf(read), TcpWriteHalf(write))
  }
}

pub struct TcpReadHalf(tokio::net::tcp::OwnedReadHalf);

impl ReadHalf for TcpReadHalf {
  async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
    self.0.read(buf).await
  }

  async fn peek(&mut self, buf: &mut [u8]) -> Result<usize> {
    self.0.peek(buf).await
  }

  fn local_addr(&self) -> Result<SocketAddr> {
    self.0.local_addr()
  }

  async fn ready(
    &self, interest: tokio::io::Interest,
  ) -> Result<tokio::io::Ready> {
    self.0.ready(interest).await
  }

  fn try_read(&self, buf: &mut [u8]) -> Result<usize> {
    self.0.try_read(buf)
  }
}

pub struct TcpWriteHalf(tokio::net::tcp::OwnedWriteHalf);

impl WriteHalf for TcpWriteHalf {
  async fn write(&mut self, buf: &[u8]) -> Result<usize> {
    self.0.write(buf).await
  }

  fn local_addr(&self) -> Result<SocketAddr> {
    self.0.local_addr()
  }

  fn peer_addr(&self) -> Result<SocketAddr> {
    self.0.peer_addr()
  }

  async fn ready(
    &self, interest: tokio::io::Interest,
  ) -> Result<tokio::io::Ready> {
    self.0.ready(interest).await
  }

  fn try_write(&self, buf: &[u8]) -> Result<usize> {
    self.0.try_write(buf)
  }

  async fn flush(&mut self) -> Result<()> {
    self.0.flush().await
  }
}
