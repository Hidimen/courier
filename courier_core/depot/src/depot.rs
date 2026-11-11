use std::{
  io::{self, ErrorKind},
  net::SocketAddr,
  sync::Arc,
  time::Duration,
};

use logger::error;
use network::transport::{StreamTransport, Transport};
use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};
use tokio::{task::JoinSet, time::sleep};

use crate::{StreamProvider, driver::drive_stream};

pub struct Depot {
  handles: JoinSet<()>,
  // ipc: LocalSocket,
  shutdown_handle: ShutdownHandle,
}

impl Depot {
  /// Creates an instance with no pods and a root shutdown token that
  /// carries `shutdown_timeout` as its grace period.
  pub fn new(shutdown_timeout: usize) -> Self {
    let shutdown_handle = ShutdownHandle::new(shutdown_timeout);

    Self {
      handles: JoinSet::new(),
      // ipc: LocalSocket::new::<DepotCommand>(shutdown_handle.child())
      //   .expect("Unable to start local socket"),
      shutdown_handle,
    }
  }

  pub fn run_stream<Pvd>(&mut self, addr: SocketAddr) -> io::Result<()>
  where
    Pvd: StreamProvider,
  {
    let transport = <Pvd::Transport as Transport>::bind(addr)?;
    let shutdown_handle_child = self.child_handle();

    self.handles.spawn(async move {
      let connection_count = Arc::new(ActivityTracker::new());

      loop {
        let res = tokio::select! {
          result = transport.accept() => result,
          _ = shutdown_handle_child.wait() => break
        };

        match res {
          Ok(stream) => {
            let protocol = Pvd::protocol();
            let child = shutdown_handle_child.child();
            let connection_count_cloned = connection_count.clone();

            tokio::spawn(async move {
              drive_stream(protocol, child, connection_count_cloned, stream)
                .await;
            });
          },
          Err(e) => match e.kind() {
            ErrorKind::ConnectionAborted => {
              error!("A connection aborted, retrying");
            },
            ErrorKind::WouldBlock => {
              continue;
            },
            ErrorKind::NetworkDown
            | ErrorKind::NetworkUnreachable
            | ErrorKind::HostUnreachable => {
              error!("A momentary mistake occurred, retrying");
              continue;
            },
            _ => {
              error!("An unrecoverable error occurred, graceful exiting");
              break;
            },
          },
        }
      }

      // Entering stopping logic
      let timeout =
        sleep(Duration::from_millis(shutdown_handle_child.timeout() as u64));

      tokio::select! {
        _ = connection_count.wait() => {},
        _ = timeout => {},
      }
    });

    Ok(())
  }

  /// Waits for the root token to be cancelled and then joins every pod.
  ///
  /// Cancelling the root token — through [`shutdown`](Self::shutdown) or
  /// through a handle obtained from [`shutdown_handle`](Self::shutdown_handle)
  /// — stops every pod at once. Each pod is joined afterwards, so this
  /// resolves only once every pod task has ended and the host may flush
  /// the event bus and the logger without losing records.
  pub async fn wait(self) {
    self.handles.join_all().await;
  }

  /// Returns the instance-wide (root) shutdown handle.
  ///
  /// Cancelling it stops every pod of this instance; this is the handle
  /// the host drives its shutdown paths with.
  pub fn shutdown_handle(&self) -> ShutdownHandle {
    self.shutdown_handle.clone()
  }

  /// Returns a shutdown handle scoped to a single pod.
  ///
  /// Cancelling it stops that pod only, leaving its siblings serving;
  /// this is the handle a pod is created with.
  pub fn child_handle(&self) -> ShutdownHandle {
    self.shutdown_handle.child()
  }
}
