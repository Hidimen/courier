use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use event::Dispatcher;
use logger::{Logger, error, info};
use network::{
  KeepAlive,
  stream::{SplitStream, StreamState, WriteHalf, check_state},
  transport::{DatagramTransport, StreamTransport},
};
use protocol::{DatagramProtocol, StreamProtocol};
use relay::Middleware;
use relay::Pipeline;
use tokio::{sync::Notify, time::sleep};

use crate::{ShutdownHandle, error::DepotError};

/// A server instance that pairs a [`Transport`](network::transport::Transport), a [`Protocol`](protocol::Protocol), a
/// [`Pipeline`] and an [`Dispatcher`].
///
/// The depot accepts connections, decodes incoming data, passes the
/// decoded context through the middleware chain, encodes the result,
/// and writes it back to the client. An event system will also be setup
/// which provide simple but compatible event bus.
///
/// # Type parameters
///
/// - `T` — the transport type.
/// - `P` — the protocol type.
/// - `M` — the composite middleware type (the chain assembled by
///   [`Builder::build`](relay::Builder::build)).
pub struct Depot<T, P, M> {
  transport: T,
  protocol: Arc<P>,
  pipeline: Arc<Pipeline<M>>,
  logger: Arc<Logger>,
  dispatcher: Arc<Dispatcher>,
  shutdown_handle: ShutdownHandle,
}

impl<T, P, M> Depot<T, P, M> {
  /// Creates a new [`Depot`] from a transport, a protocol, a pipeline, a
  /// logger, an event bus, and the [`CancellationToken`] that controls its
  /// shutdown.
  pub fn new(
    transport: T, protocol: P, pipeline: Pipeline<M>, logger: Arc<Logger>,
    dispatcher: Arc<Dispatcher>, shutdown_timeout: usize,
  ) -> Self {
    Self {
      transport,
      protocol: Arc::new(protocol),
      pipeline: Arc::new(pipeline),
      logger,
      dispatcher,
      shutdown_handle: ShutdownHandle::new(shutdown_timeout),
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

  /// Returns a cloned logger.
  pub fn logger(&self) -> Arc<Logger> {
    self.logger.clone()
  }

  /// Returns a cloned dispatcher.
  pub fn dispatcher(&self) -> Arc<Dispatcher> {
    self.dispatcher.clone()
  }

  pub(crate) fn shutdown_handle(&self) -> ShutdownHandle {
    self.shutdown_handle.clone()
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
  pub async fn run_stream(self) {
    let connection_count = Arc::new(AtomicUsize::new(0));
    let shutdown = self.shutdown_handle.child();
    let notify = Arc::new(Notify::new());

    loop {
      let accept_result = tokio::select! {
        result = self.transport.accept() => result,
        _ = shutdown.wait() => break
      };

      let (mut read, mut write) = match accept_result {
        Ok(s) => s.split(),
        Err(e) => {
          error!("IO error: {}", e);
          continue;
        },
      };
      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      let child = shutdown.child();
      let count = connection_count.clone();
      let notifier = notify.clone();
      tokio::spawn(async move {
        let mut request_count: usize = 0;
        let mut keep_alive = KeepAlive::Pending;
        let count = count;
        count.fetch_add(1, Ordering::Relaxed);

        loop {
          if child.is_shutdown() {
            break;
          }

          let state = check_state(&mut read, &mut write).await;

          match state {
            StreamState::Open => {
              let req = match keep_alive {
                KeepAlive::Timeout(d) => {
                  match tokio::time::timeout(d, protocol.decode(&mut read))
                    .await
                  {
                    Ok(Ok(req)) => req,
                    Ok(Err(e)) => {
                      error!("An error emitted by protocol decoder: {}", e);
                      break;
                    },
                    Err(_elapsed) => break,
                  }
                },
                _ => match protocol.decode(&mut read).await {
                  Ok(req) => req,
                  Err(e) => {
                    error!("An error emitted by protocol decoder: {}", e);
                    break;
                  },
                },
              };

              request_count += 1;

              let resp = match pipeline.handle(req).await {
                Ok(resp) => resp,
                Err(e) => {
                  error!("An error emitted by middleware handler: {}", e);
                  break;
                },
              };

              let (frame, new_keep_alive) = match protocol.encode(resp).await {
                Ok(result) => result,
                Err(e) => {
                  error!("An error emitted by protocol encoder: {}", e);
                  break;
                },
              };

              match write.write_all(frame.as_ref()).await {
                Ok(_) => {},
                Err(e) => {
                  error!("An error occurred when sending data: {}", e);
                  break;
                },
              }

              keep_alive = new_keep_alive;

              match keep_alive {
                KeepAlive::Close => break,
                KeepAlive::UpTo(n) if request_count >= n => break,
                _ => continue,
              }
            },
            StreamState::PeerClosed => {
              let req = match keep_alive {
                KeepAlive::Timeout(d) => {
                  match tokio::time::timeout(d, protocol.decode(&mut read))
                    .await
                  {
                    Ok(Ok(req)) => req,
                    Ok(Err(e)) => {
                      error!("An error emitted by protocol decoder: {}", e);
                      break;
                    },
                    Err(_elapsed) => break,
                  }
                },
                _ => match protocol.decode(&mut read).await {
                  Ok(req) => req,
                  Err(e) => {
                    error!("An error emitted by protocol decoder: {}", e);
                    break;
                  },
                },
              };

              let resp = match pipeline.handle(req).await {
                Ok(resp) => resp,
                Err(e) => {
                  error!("An error emitted by middleware handler: {}", e);
                  break;
                },
              };

              let (frame, _) = match protocol.encode(resp).await {
                Ok(result) => result,
                Err(e) => {
                  error!("An error emitted by protocol encoder: {}", e);
                  break;
                },
              };

              match write.write_all(frame.as_ref()).await {
                Ok(_) => {},
                Err(e) => {
                  error!("An error occurred when sending data: {}", e);
                  break;
                },
              }

              break;
            },
            _ => break,
          };
        }

        if count.fetch_sub(1, Ordering::Release) == 1 {
          notifier.notify_one();
        }
      });
    }

    // Cleanup
    let timeout = self.shutdown_handle.timeout();
    info!(
      "Graceful shutdown starting. Forced termination will occur in {} ms",
      timeout
    );

    let count_cloned = connection_count.clone();
    let notify_cloned = notify.clone();

    tokio::spawn(async move {
      sleep(Duration::from_millis(timeout as u64)).await;
      count_cloned.store(0, Ordering::Release);
      notify_cloned.notify_one();
    });

    if connection_count.load(Ordering::Acquire) > 0 {
      info!("Waiting for all tasks to complete");
      while connection_count.load(Ordering::Acquire) > 0 {
        notify.notified().await;
      }
    }

    info!("Server stopped");
  }
}

impl<T, P, M> Depot<T, P, M>
where
  T: DatagramTransport,
  P: DatagramProtocol<T>,
  M: Middleware<P::Request, Response = P::Response> + Send + Sync + 'static,
  M::Error: Send + 'static,
  P::Error: Send + Sync,
{
  /// Runs the datagram server loop.
  ///
  /// For each received datagram, this decodes the bytes, passes the
  /// decoded context through the middleware chain, encodes the result,
  /// and sends it back to the peer.
  pub async fn run_datagram(self) {
    let shutdown = self.shutdown_handle.child();
    loop {
      let mut buf = [0u8; 1024];
      let recv_result = tokio::select! {
        result = self.transport.recv_from(&mut buf) => result,
        _ = shutdown.wait() => break
      };
      let (_, peer) = match recv_result {
        Ok(d) => d,
        Err(e) => {
          error!("IO error: {}", e);
          continue;
        },
      };

      let protocol = self.protocol.clone();
      let pipeline = self.pipeline.clone();

      let data: Result<network::Frame, DepotError> =
        match tokio::spawn(async move {
          let ctx = protocol.decode(&buf).await.map_err(|e| {
            error!("An error emitted by protocol decoder: {}", e);
            DepotError::Execution
          })?;
          let ctx = pipeline.handle(ctx).await.map_err(|e| {
            error!("An error emitted by middleware handler: {}", e);
            DepotError::Execution
          })?;
          let frame = protocol.encode(ctx).await.map_err(|e| {
            error!("An error emitted by protocol encoder: {}", e);
            DepotError::Execution
          })?;
          Ok(frame)
        })
        .await
        {
          Ok(d) => d,
          Err(e) => {
            error!("Task terminated: {}", e);
            continue;
          },
        };

      match data {
        Ok(frame) => match self.transport.send_to(frame.as_ref(), peer).await {
          Ok(_) => continue,
          Err(e) => {
            error!("Failed to send data to peer: {}", e);
            continue;
          },
        },
        Err(DepotError::Execution) => {
          continue;
        },
        _ => unreachable!(),
      };
    }

    info!("Server stopped");
  }
}
