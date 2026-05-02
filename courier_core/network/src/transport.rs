pub mod tcp;

use std::{
  io::Result,
  net::{SocketAddr, ToSocketAddrs},
};

use async_trait::async_trait;

use crate::stream::{ReadHalf, SplitStream};

pub trait Transport: Send + Sync + Sized + 'static {
  type Data: Send + Sync + 'static;

  fn bind<A: ToSocketAddrs + Send>(addr: A) -> Result<Self>;
}

#[async_trait]
pub trait StreamTransport: Transport
where
  Self::Data: ReadHalf,
{
  type Stream: SplitStream + 'static;

  async fn accept(&self) -> Result<Self::Stream>;

  fn set_ttl(&self, ttl: u32) -> Result<()>;

  fn ttl(&self) -> Result<u32>;

  fn local_addr(&self) -> Result<SocketAddr>;
}

#[async_trait]
pub trait DatagramTransport: Transport {
  async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

  async fn send(&self, buf: &[u8]) -> Result<usize>;

  fn set_ttl(&self, ttl: u32) -> Result<()>;

  fn ttl(&self) -> Result<u32>;

  fn local_addr(&self) -> Result<SocketAddr>;

  fn set_nodelay(&self, nodelay: bool) -> Result<()>;

  fn nodelay(&self) -> Result<bool>;
}
