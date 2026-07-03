use std::sync::Arc;

use logger::Logger;
use network::{
  stream::{SplitStream, WriteHalf},
  transport::{DatagramTransport, StreamTransport},
};
use pipeline::Middleware;
use pipeline::Pipeline;
use protocol::{DatagramProtocol, StreamProtocol};

use crate::error::DepotError;

/// A server instance that pairs a [`Transport`], a [`Protocol`], and a
/// [`Pipeline`].
///
/// The depot accepts connections, decodes incoming data, passes the
/// decoded context through the middleware chain, encodes the result,
/// and writes it back to the client.
///
/// # Type parameters
///
/// - `T` — the transport type.
/// - `P` — the protocol type.
/// - `M` — the composite middleware type (the chain assembled by
///   [`Builder::build`](pipeline::Builder::build)).
pub struct Depot<T, P, M> {
  transport: T,
  protocol: Arc<P>,
  pipeline: Arc<Pipeline<M>>,
  logger: Arc<Logger>,
}

impl<T, P, M> Depot<T, P, M> {
  /// Creates a new [`Depot`] from a transport, a protocol, and a
  /// pipeline.
  pub fn new(
    transport: T, protocol: P, pipeline: Pipeline<M>, logger: Arc<Logger>,
  ) -> Self {
    logger.install();
    Self {
      transport,
      protocol: Arc::new(protocol),
      pipeline: Arc::new(pipeline),
      logger,
    }
  }

  /// Returns a reference to the transport.
  pub fn transport(&self) -> &T {
    &self.transport
  }

  /// Returns a reference to the pipeline.
  pub fn pipeline(&self) -> &Pipeline<M> {
    &self.pipeline
  }

  pub fn logger(&self) -> Arc<Logger> {
    self.logger.clone()
  }
}

impl<T, P, M> Depot<T, P, M>
where
  T: StreamTransport,
  P: StreamProtocol<T>,
  M: Middleware<P::Request, Response = P::Response> + Send + Sync + 'static,
  M::Error: Send + 'static,
  P::Error: Into<M::Error>,
{
  /// Runs the stream server loop.
  ///
  /// For each accepted connection, this decodes the incoming stream,
  /// passes the decoded context through the middleware chain, encodes
  /// the result, and writes it back to the client.
  pub async fn run_stream(&mut self) -> Result<(), DepotError<M::Error>> {
    loop {
      let (read, mut write) = self.transport.accept().await?.split();
      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      let data: Result<network::Frame, DepotError<M::Error>> =
        match tokio::spawn(async move {
          let ctx =
            protocol.decode(read).await.map_err(ProtocolErrorWrapper)?;
          let ctx = pipeline.handle(ctx).await.map_err(DepotError::Process)?;
          let frame = protocol.encode(ctx).await;
          Ok(frame)
        })
        .await
        {
          Ok(d) => d,
          Err(e) => return Err(DepotError::TaskTerminated(e)),
        };
      write.write_all(data?.as_ref()).await?;
      write.flush().await?
    }
  }
}

impl<T, P, M> Depot<T, P, M>
where
  T: DatagramTransport,
  P: DatagramProtocol<T>,
  M: Middleware<P::Request, Response = P::Response> + Send + Sync + 'static,
  M::Error: Send + 'static,
  P::Error: Into<M::Error>,
{
  /// Runs the datagram server loop.
  ///
  /// For each received datagram, this decodes the bytes, passes the
  /// decoded context through the middleware chain, encodes the result,
  /// and sends it back to the peer.
  pub async fn run_datagram(&mut self) -> Result<(), DepotError<M::Error>> {
    loop {
      let mut buf = [0u8; 1024];
      let (_, peer) = self.transport.recv_from(&mut buf).await?;

      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      let data: Result<network::Frame, DepotError<M::Error>> =
        match tokio::spawn(async move {
          let ctx =
            protocol.decode(&buf).await.map_err(ProtocolErrorWrapper)?;
          let ctx = pipeline.handle(ctx).await.map_err(DepotError::Process)?;
          let frame = protocol.encode(ctx).await;
          Ok(frame)
        })
        .await
        {
          Ok(d) => d,
          Err(e) => return Err(DepotError::TaskTerminated(e)),
        };

      self.transport.send_to(data?.as_ref(), peer).await?;
    }
  }
}

/// A thin adapter that wraps a protocol error so it can be used inside
/// a spawned task alongside middleware errors.
#[derive(Debug)]
struct ProtocolErrorWrapper<E>(pub E);

impl<E: std::fmt::Display + std::error::Error + 'static> std::fmt::Display
  for ProtocolErrorWrapper<E>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl<E: std::fmt::Display + std::error::Error + 'static> std::error::Error
  for ProtocolErrorWrapper<E>
{
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.0)
  }
}

impl<E, ME> From<ProtocolErrorWrapper<E>> for DepotError<ME>
where
  E: std::error::Error + Send + Sync + 'static + Into<ME>,
  ME: std::error::Error + Send + Sync + 'static,
{
  fn from(value: ProtocolErrorWrapper<E>) -> Self {
    DepotError::Process(value.0.into())
  }
}
