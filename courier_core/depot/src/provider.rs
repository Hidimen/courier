//! Provider definitions and the capabilities bound to a
//! [`Depot`](crate::Depot).
//!
//! A builtin module declares the transport and protocol it serves by
//! implementing [`StreamProvider`] or [`DatagramProvider`]. The capability
//! abstraction itself lives in `extension`; this module is the bridge that
//! fixes its runtime type to [`Depot`](crate::Depot).

use std::{io, net::SocketAddr};

use extension::builtin::capability::*;
use network::transport::{DatagramTransport, StreamTransport};
use protocol::{Protocol, datagram::DatagramProtocol, stream::StreamProtocol};

use crate::Depot;

/// The transport and protocol a stream builtin module provides.
///
/// # Examples
///
/// ```rust
/// use depot::StreamProvider;
///
/// fn transport<P: StreamProvider>() -> std::any::TypeId {
///   std::any::TypeId::of::<<P as StreamProvider>::Transport>()
/// }
/// ```
pub trait StreamProvider: Sized {
  /// The stream transport this provider requires, e.g. `TcpTransport`.
  type Transport: StreamTransport;

  /// The protocol, which must run over [`Self::Transport`].
  type Protocol: Protocol + StreamProtocol;

  fn protocol() -> Self::Protocol;
}

/// The transport and protocol a datagram builtin module provides.
///
/// # Examples
///
/// ```rust
/// use depot::DatagramProvider;
///
/// fn transport<P: DatagramProvider>() -> std::any::TypeId {
///   std::any::TypeId::of::<<P as DatagramProvider>::Transport>()
/// }
/// ```
pub trait DatagramProvider: Sized {
  /// The datagram transport this provider requires, e.g. `UdpTransport`.
  type Transport: DatagramTransport;

  /// The protocol, which must run over [`Self::Transport`].
  type Protocol: Protocol + DatagramProtocol;

  fn protocol() -> Self::Protocol;
}

/// Creates a stream provider capability served by [`Depot`].
///
/// # Examples
///
/// ```rust,no_run
/// use depot::provider;
/// # use network::transport::tcp::TcpTransport;
/// # use protocol::{Protocol, stream::StreamProtocol};
/// # struct Demo;
/// # impl Protocol for Demo {
/// #   type Command = ();
/// #   type Error = std::io::Error;
/// #   const NAME: &'static str = "demo";
/// #   fn create() -> Self { Self }
/// # }
/// # impl StreamProtocol for Demo {
/// #   fn step(
/// #     &mut self, _: &mut protocol::stream::Context<'_>,
/// #     _: protocol::stream::Input<()>,
/// #   ) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # struct DemoProvider;
/// # impl depot::StreamProvider for DemoProvider {
/// #   type Transport = TcpTransport;
/// #   type Protocol = Demo;
/// # }
/// let capability = provider::stream::<DemoProvider>();
/// ```
pub fn stream<P: StreamProvider>() -> Provider<Depot> {
  fn install<P: StreamProvider>(
    depot: &mut Depot, address: SocketAddr,
  ) -> io::Result<()> {
    depot.run_stream::<P>(address)
  }

  Provider::new(install::<P>)
}

/// Creates a datagram provider capability served by [`Depot`].
///
/// # Examples
///
/// ```rust,no_run
/// use depot::provider;
/// # use network::transport::udp::UdpTransport;
/// # use protocol::{Protocol, datagram::{Context, DatagramProtocol, Input}};
/// # struct Demo;
/// # impl Protocol for Demo {
/// #   type Command = ();
/// #   type Error = std::io::Error;
/// #   const NAME: &'static str = "demo";
/// #   fn create() -> Self { Self }
/// # }
/// # impl DatagramProtocol for Demo {
/// #   fn step(
/// #     &mut self, _: &mut Context<'_>, _: Input<()>,
/// #   ) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # struct DemoProvider;
/// # impl depot::DatagramProvider for DemoProvider {
/// #   type Transport = UdpTransport;
/// #   type Protocol = Demo;
/// # }
/// let capability = provider::datagram::<DemoProvider>();
/// ```
pub fn datagram<P: DatagramProvider>() -> Provider<Depot> {
  fn install<P: DatagramProvider>(
    depot: &mut Depot, address: SocketAddr,
  ) -> io::Result<()> {
    depot.run_datagram::<P>(address)
  }

  Provider::new(install::<P>)
}
