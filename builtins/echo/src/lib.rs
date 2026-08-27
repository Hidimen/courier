use depot::{self, StreamProvider};
use network::{
  Frame, KeepAlive, stream::ReadHalf, transport::tcp::TcpTransport,
};
use protocol::{Protocol, StreamProtocol};
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
pub struct Request(pub String);

/// A response carrying the request payload back to the peer.
pub struct Response(pub String);

impl Protocol for EchoProtocol {
  type Error = EchoError;
  type Request = Request;
  type Response = Response;

  fn name() -> &'static str {
    "protocol:echo"
  }

  fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
  }
}

impl StreamProtocol<TcpTransport> for EchoProtocol {
  async fn decode(
    &self,
    read_half: &mut <TcpTransport as network::transport::StreamTransport>::ReadHalf,
  ) -> Result<Self::Request, Self::Error> {
    let mut buf = [0u8; 1024];
    let _ = read_half.read(&mut buf).await?;

    Ok(Request(String::from_utf8_lossy(&buf).to_string()))
  }

  async fn encode(
    &self, response: Self::Response,
  ) -> Result<(network::Frame, network::KeepAlive), Self::Error> {
    Ok((Frame::from_owner(response.0), KeepAlive::Keep))
  }
}

/// The echo builtin parts: a protocol plus a middleware pipeline.
///
/// The transport, logger and event bus are created by the framework from
/// the server [`Config`](depot::config::Config); this type only declares
/// the transport its protocol runs over and supplies the pipeline.
#[derive(Default, Clone, Copy)]
pub struct EchoProvider;

impl StreamProvider for EchoProvider {
  type Transport = TcpTransport;
  type Protocol = EchoProtocol;

  fn protocol(&self) -> Self::Protocol {
    EchoProtocol
  }

  fn pipeline(
    &self,
  ) -> relay::Pipeline<impl relay::Middleware<Request, Response = Response>> {
    relay::Builder::default()
      .then(|request: Request| Response(request.0))
      .build()
  }
}
