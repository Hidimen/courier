use depot::DatagramProvider;
use network::{
  Frame, transport::udp::UdpTransport,
};
use protocol::{DatagramProtocol, Protocol};
use thiserror::Error;

/// Error type for the echo protocol.
#[derive(Error, Debug)]
pub enum EchoError {
  /// An I/O error from the underlying stream.
  #[error("IO Error")]
  IOError(#[from] std::io::Error),
}

/// The echo protocol.
#[derive(Clone, Copy)]
pub struct EchoProtocol;

/// A request carrying the received bytes as a string.
pub struct Request(pub Frame);

/// A response carrying the request payload back to the peer.
pub struct Response(pub Frame);

impl Protocol for EchoProtocol {
  type Error = EchoError;
  type Request = Request;
  type Response = Response;

  fn name() -> &'static str {
    "protocol:echo_udp"
  }

  fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
  }
}

impl DatagramProtocol<UdpTransport> for EchoProtocol {
  async fn decode(
      &self, data: (Frame, std::net::SocketAddr),
    ) -> Result<Self::Request, Self::Error>
  {
    Ok(Request(data.0))
  }

  async fn encode(
      &self, response: Self::Response,
    ) -> Result<Frame, Self::Error>
  {
    Ok(response.0)
  }
}

/// The echo builtin parts: a protocol plus a middleware pipeline.
///
/// The transport, logger and event bus are created by the framework from
/// the server [`Config`](depot::config::Config); this type only declares
/// the transport its protocol runs over and supplies the pipeline.
pub struct EchoProvider;

impl DatagramProvider for EchoProvider {
  type Transport = UdpTransport;
  type Protocol = EchoProtocol;

  fn protocol(&self) -> Self::Protocol {
    EchoProtocol
  }

  fn create() -> Self {
    Self
  }

  fn pipeline(
    &self,
  ) -> relay::Pipeline<impl relay::Middleware<Request, Response = Response>> {
    relay::Builder::default()
      .then(|request: Request| Response(request.0))
      .build()
  }
}
