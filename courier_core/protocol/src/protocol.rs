use std::error::Error;

use network::{
  Frame, KeepAlive,
  transport::{DatagramTransport, StreamTransport},
};

/// A protocol that defines how bytes are converted to and from
/// application-level requests and responses.
///
/// Each protocol implementation defines its own [`Error`](Self::Error)
/// type for encoding and decoding failures.
pub trait Protocol: Send + Sync + Sized + 'static {
  /// The error type returned by encoding and decoding operations.
  type Error: Error + Send + Sync + 'static;
  /// The application-level request type.
  type Request: Send + 'static;
  /// The application-level response type.
  type Response: Send + 'static;

  /// Returns the name of this protocol (e.g. `"HTTP"`).
  fn name() -> &'static str;
  /// Returns the protocol version (e.g. `"1.1"`).
  fn version() -> &'static str;
}

/// A [`Protocol`] that operates on byte streams.
pub trait StreamProtocol<T: StreamTransport>: Protocol {
  /// Decodes a request from the stream.
  fn decode(
    &self, read_half: &mut T::ReadHalf,
  ) -> impl Future<Output = Result<Self::Request, Self::Error>> + Send;

  /// Encodes a response into a frame and keep-alive directive.
  fn encode(
    &self, response: Self::Response,
  ) -> impl Future<Output = Result<(Frame, KeepAlive), Self::Error>> + Send;
}

/// A [`Protocol`] that operates on datagrams.
pub trait DatagramProtocol<T: DatagramTransport>: Protocol {
  /// Decodes a request from a datagram payload.
  fn decode(
    &self, data: &[u8],
  ) -> impl Future<Output = Result<Self::Request, Self::Error>> + Send;

  /// Encodes a response into a frame and keep-alive directive.
  fn encode(
    &self, response: Self::Response,
  ) -> impl Future<Output = Result<(Frame, KeepAlive), Self::Error>> + Send;
}
