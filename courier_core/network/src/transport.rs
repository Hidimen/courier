pub mod tcp;
pub mod udp;

use std::{
  io::Result,
  net::{SocketAddr, ToSocketAddrs},
};

use async_trait::async_trait;

use crate::stream::{ReadHalf, SplitStream, WriteHalf};

pub trait Transport: Send + Sync + Sized + 'static {
  fn bind<A: ToSocketAddrs + Send>(addr: A) -> Result<Self>;

  fn set_ttl(&self, ttl: u32) -> Result<()>;

  fn ttl(&self) -> Result<u32>;

  fn local_addr(&self) -> Result<SocketAddr>;
}

#[async_trait]
pub trait StreamTransport: Transport {
  type ReadHalf: ReadHalf + 'static;
  type WriteHalf: WriteHalf + 'static;
  type Stream: SplitStream<ReadHalf = Self::ReadHalf, WriteHalf = Self::WriteHalf> + 'static;

  async fn accept(&self) -> Result<Self::Stream>;
}

#[async_trait]
pub trait DatagramTransport: Transport {
  async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

  async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;

  async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize>;
}
