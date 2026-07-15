use std::sync::Arc;

use logger::{Logger, error, warn};
use network::{
  KeepAlive,
  stream::{SplitStream, StreamState, WriteHalf, check_state},
  transport::{DatagramTransport, StreamTransport},
};
use protocol::{DatagramProtocol, StreamProtocol};
use relay::Middleware;
use relay::Pipeline;

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
{
  /// Runs the stream server loop.
  ///
  /// For each accepted connection, this decodes the incoming stream,
  /// passes the decoded context through the middleware chain, encodes
  /// the result, and writes it back to the client.
  ///
  /// The inner loop respects keep-alive directives returned by the
  /// protocol's [`encode`](StreamProtocol::encode):
  ///
  /// - [`KeepAlive::Keep`] — continue processing requests on this
  ///   connection indefinitely.
  /// - [`KeepAlive::Close`] — close the connection after the current
  ///   response.
  /// - [`KeepAlive::UpTo`] — process at most `n` requests, then close.
  /// - [`KeepAlive::Timeout`] — wrap each subsequent decode in a
  ///   timeout; if no request arrives within the duration, close the
  ///   connection.
  /// - [`KeepAlive::Pending`] — treated as [`KeepAlive::Keep`]; the
  ///   protocol hasn't decided yet.
  pub async fn run_stream(&mut self) {
    loop {
      let (mut read, mut write) = match self.transport.accept().await {
        Ok(s) => s.split(),
        Err(e) => {
          error!("main", "IO error: {}", e);
          continue;
        },
      };
      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      tokio::spawn(async move {
        let mut request_count: usize = 0;
        let mut keep_alive = KeepAlive::Pending;

        loop {
          // Probe the connection state before attempting a decode.
          // If the peer has already closed or the stream is broken,
          // stop this connection handler.
          match check_state(&mut read, &mut write).await {
            StreamState::Open | StreamState::HalfOpen => {},
            _ => break,
          }

          // Decode the next request. When the keep-alive directive is
          // `Timeout`, wrap the decode future so the task does not
          // wait indefinitely for a new request.
          let req = match keep_alive {
            KeepAlive::Timeout(d) => {
              match tokio::time::timeout(d, protocol.decode(&mut read)).await {
                Ok(Ok(req)) => req,
                Ok(Err(e)) => {
                  warn!("main", "An error emitted by protocol decoder: {}", e);
                  break;
                },
                Err(_elapsed) => break,
              }
            },
            _ => match protocol.decode(&mut read).await {
              Ok(req) => req,
              Err(e) => {
                warn!("main", "An error emitted by protocol decoder: {}", e);
                break;
              },
            },
          };

          request_count += 1;

          // Pass the request through the middleware pipeline.
          let resp = match pipeline.handle(req).await {
            Ok(resp) => resp,
            Err(e) => {
              warn!("main", "An error emitted by middleware handler: {}", e);
              break;
            },
          };

          // Encode the response and obtain the updated keep-alive
          // directive from the protocol.
          let (frame, new_keep_alive) = match protocol.encode(resp).await {
            Ok(result) => result,
            Err(e) => {
              warn!("main", "An error emitted by protocol encoder: {}", e);
              break;
            },
          };

          match write.write_all(frame.as_ref()).await {
            Ok(_) => {},
            Err(e) => {
              warn!("main", "An error occurred when sending data: {}", e);
              break;
            },
          }

          keep_alive = new_keep_alive;

          // Apply the keep-alive directive to decide whether to
          // continue processing on this connection.
          match keep_alive {
            KeepAlive::Close => break,
            KeepAlive::UpTo(n) if request_count >= n => break,
            _ => continue,
          }
        }
      });
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
  pub async fn run_datagram(&mut self) {
    loop {
      let mut buf = [0u8; 1024];
      let (_, peer) = match self.transport.recv_from(&mut buf).await {
        Ok(d) => d,
        Err(e) => {
          error!("main", "IO error: {}", e);
          continue;
        },
      };

      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      let data: Result<network::Frame, DepotError> =
        match tokio::spawn(async move {
          let ctx = protocol.decode(&buf).await.map_err(|e| {
            error!("main", "An error emitted by protocol decoder: {}", e);
            DepotError::Execution
          })?;
          let ctx = pipeline.handle(ctx).await.map_err(|e| {
            error!("main", "An error emitted by middleware handler: {}", e);
            DepotError::Execution
          })?;
          let (frame, _keep_alive) =
            protocol.encode(ctx).await.map_err(|e| {
              error!("main", "An error emitted by protocol encoder: {}", e);
              DepotError::Execution
            })?;
          Ok(frame)
        })
        .await
        {
          Ok(d) => d,
          Err(e) => {
            error!("main", "Task terminated: {}", e);
            continue;
          },
        };

      match data {
        Ok(frame) => match self.transport.send_to(frame.as_ref(), peer).await {
          Ok(_) => continue,
          Err(e) => {
            error!("main", "Failed to send data to peer: {}", e);
            continue;
          },
        },
        Err(DepotError::Execution) => {
          continue;
        },
        _ => unreachable!(),
      };
    }
  }
}
