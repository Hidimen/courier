use std::net::SocketAddr;

use depot::{DatagramProvider, Depot, provider::StreamProvider};
use extension::builtin::{Module, Registry};
use network::transport::{tcp::TcpTransport, udp::UdpTransport};
use protocol::{Payload, Protocol, stream, datagram};
use thiserror::Error;

/// Error type for the echo protocol.
#[derive(Error, Debug)]
pub enum EchoError {
  /// An I/O error from the underlying stream.
  #[error("IO Error")]
  IOError(#[from] std::io::Error),
}

/// The echo protocol in `stream` mode.
#[derive(Clone)]
pub struct EchoStreamProtocol {
  buf: Vec<Payload>,
}

impl Protocol for EchoStreamProtocol {
  type Error = EchoError;
  type Command = ();

  const NAME: &'static str = "courier:echo";
}

impl stream::StreamProtocol for EchoStreamProtocol {
  fn step(
    &mut self, ctx: &mut stream::Context<'_>, input: stream::Input<Self::Command>,
  ) -> Result<(), Self::Error> {
    match input {
      stream::Input::Opened => Ok(()),
      stream::Input::Closed(_) => Ok(()),
      stream::Input::Data(bytes) => {
        if let Err(payload) = ctx.write_bytes(bytes) {
          self.buf.push(payload);
          Ok(())
        } else {
          Ok(())
        }
      },
      stream::Input::Command(_) => Ok(()),
      stream::Input::Timer(_) => Ok(()),
    }
  }
}

/// The echo protocol in `datagram` mode.
#[derive(Clone)]
pub struct EchoDatagramProtocol {
  buf: Vec<(SocketAddr, Payload)>,
}

impl Protocol for EchoDatagramProtocol {
  type Error = EchoError;
  type Command = ();

  const NAME: &'static str = "courier:echo_datagram";
}

impl datagram::DatagramProtocol for EchoDatagramProtocol {
  fn step(
    &mut self, ctx: &mut datagram::Context<'_>, input: datagram::Input<Self::Command>,
  ) -> Result<(), Self::Error> {
    match input {
      datagram::Input::Opened => Ok(()),
      datagram::Input::Closed(_) => Ok(()),
      datagram::Input::Message { peer, payload } => {
        if let Err(payload) = ctx.write_bytes(peer, payload) {
          self.buf.push(payload);
          Ok(())
        } else {
          Ok(())
        }
      },
      datagram::Input::Command(_) => Ok(()),
      datagram::Input::Timer(_) => Ok(()),
    }
  }
}

/// The echo stream provider.
///
/// The transport is created by the framework from the configured listen
/// address; this type only declares the transport its protocol runs over.
pub struct EchoProvider;

impl StreamProvider for EchoProvider {
  type Transport = TcpTransport;
  type Protocol = EchoStreamProtocol;

  fn protocol() -> Self::Protocol {
    Self::Protocol {
      buf: Vec::new()
    }
  }
}

impl DatagramProvider for EchoProvider {
  type Transport = UdpTransport;
  type Protocol = EchoDatagramProtocol;

  fn protocol() -> Self::Protocol {
    Self::Protocol {
      buf: Vec::new()
    }
  }
}

/// The TCP echo builtin module.
pub struct Echo;

pub fn builtin_module_entry(registry: &mut Registry<Depot>) {
  registry.register(
    Module::new(
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_VERSION"),
      env!("CARGO_PKG_DESCRIPTION"),
    )
    .stream_provider(depot::provider::stream::<EchoProvider>())
    .datagram_provider(depot::provider::datagram::<EchoProvider>()),
  );
}
