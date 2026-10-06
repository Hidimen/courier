pub mod tcp;
pub mod udp;

use std::{io::Result, net::SocketAddr};

use crate::stream::{ReadHalf, Stream, WriteHalf};

pub trait Transport: Send + Sync + Sized + 'static {
  fn bind(addr: SocketAddr) -> Result<Self>;

  #[cfg(not(windows))]
  fn bind_reuse(addr: SocketAddr) -> Result<Self>;

  fn set_ttl(&self, ttl: u32) -> Result<()>;

  fn ttl(&self) -> Result<u32>;

  fn local_addr(&self) -> Result<SocketAddr>;
}

pub trait StreamTransport: Transport {
  type ReadHalf: ReadHalf + 'static;
  type WriteHalf: WriteHalf + 'static;
  type Stream: Stream<ReadHalf = Self::ReadHalf, WriteHalf = Self::WriteHalf>
    + 'static;

  fn accept(&self) -> impl Future<Output = Result<Self::Stream>> + Send;
}

pub trait DatagramTransport: Transport {
  fn recv_from(
    &self, buf: &mut [u8],
  ) -> impl Future<Output = Result<(usize, SocketAddr)>> + Send;

  fn send_to(
    &self, buf: &[u8], addr: SocketAddr,
  ) -> impl Future<Output = Result<usize>> + Send;
}
