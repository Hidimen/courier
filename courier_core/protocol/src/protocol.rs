use network::{
  Frame,
  transport::{DatagramTransport, StreamTransport},
};

use crate::error::ProtocolError;

pub trait Protocol: Send + Sync + Sized + 'static {
  type Error: ProtocolError;
  type Request: Send + 'static;
  type Response: Send + 'static;

  fn name() -> &'static str;
  fn version() -> &'static str;
}

pub trait StreamProtocol<T: StreamTransport>: Protocol {
  fn decode(
    &self, data: &mut T::ReadHalf,
  ) -> impl Future<Output = Result<Self::Request, Self::Error>> + Send;

  fn encode(&self, data: Self::Response) -> impl Future<Output = Frame> + Send;
}

pub trait DatagramProtocol<T: DatagramTransport>: Protocol {
  fn decode(
    &self, data: &[u8],
  ) -> impl Future<Output = Result<Self::Request, Self::Error>> + Send;

  fn encode(&self, data: Self::Response) -> impl Future<Output = Frame> + Send;
}
