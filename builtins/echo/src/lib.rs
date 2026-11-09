use depot::{self, StreamProvider};
use network::{
  transport::tcp::TcpTransport,
};
use protocol::{Payload, Protocol, stream::*};
use thiserror::Error;

/// Error type for the echo protocol.
#[derive(Error, Debug)]
pub enum EchoError {
  /// An I/O error from the underlying stream.
  #[error("IO Error")]
  IOError(#[from] std::io::Error),
}

/// The echo protocol.
#[derive(Clone)]
pub struct EchoProtocol {
  buf: Vec<Payload>
}

/// A request carrying the received bytes as a string.
pub struct Request(pub Vec<u8>);

/// A response carrying the request payload back to the peer.
pub struct Response(pub Vec<u8>);

impl Protocol for EchoProtocol {
  type Error = EchoError;
  type Command = ();
  
  const NAME: &'static str = "courier:echo";
}

impl StreamProtocol for EchoProtocol {
  fn step(
      &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
    ) -> Result<(), Self::Error>
  {
    match input {
      Input::Opened => Ok(()),
      Input::Closed(_) => Ok(()),
      Input::Data(bytes) => {
        if let Err(payload) = ctx.write_bytes(bytes) {
          self.buf.push(payload);
          Ok(())
        }else {
          Ok(())
        }
      },
      Input::Command(_) => Ok(()),
      Input::Timer(_) => Ok(())
    }
  }
}

/// The echo builtin parts: a protocol plus a middleware pipeline.
///
/// The transport, logger and event bus are created by the framework from
/// the server [`Config`](depot::config::Config); this type only declares
/// the transport its protocol runs over and supplies the pipeline.
pub struct EchoProvider;

impl StreamProvider for EchoProvider {
  type Transport = TcpTransport;
  type Protocol = EchoProtocol;

  fn protocol() -> Self::Protocol {
    EchoProtocol { buf: Vec::new() }
  }
}
