use async_trait::async_trait;
use network::{
  Context, Frame,
  transport::{DatagramTransport, StreamTransport},
};

use crate::error::ProtocolError;

pub trait Protocol: Send + Sync + Sized + 'static {
  type Error: ProtocolError;
  type Context: Context;

  fn name() -> &'static str;
  fn version() -> &'static str;
}

#[async_trait]
pub trait StreamProtocol<T: StreamTransport>: Protocol {
  type Data: Sized + 'static;

  async fn decode(&self, data: T::ReadHalf) -> Result<Self::Data, Self::Error>;

  async fn encode(&self, data: Self::Data) -> Frame;
}

#[async_trait]
pub trait DatagramProtocol<T: DatagramTransport>: Protocol {
  type Data: Sized + 'static;

  async fn decode(&self, data: &[u8]) -> Result<Self::Data, Self::Error>;

  async fn encode(&self, data: Self::Data) -> Frame;
}
