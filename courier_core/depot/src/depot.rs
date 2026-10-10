use std::{io, net::SocketAddr};

use ipc::LocalSocket;
use network::transport::{DatagramTransport, StreamTransport};
use protocol::{DatagramProtocol, StreamProtocol};

use crate::{
  DatagramProvider, Pod, ShutdownHandle, StreamProvider, control::DepotCommand,
};

pub struct Depot {
  pods: Vec<Pod>,
  shutdown_handle: ShutdownHandle,
  current_id: usize,
  ipc: LocalSocket,
}

impl Depot {
  /// Creates an instance with no pods and a root shutdown token that
  /// carries `shutdown_timeout` as its grace period.
  pub fn new(shutdown_timeout: usize) -> Self {
    Self {
      pods: Vec::new(),
      shutdown_handle: ShutdownHandle::new(shutdown_timeout),
      current_id: 0,
      ipc: LocalSocket::new::<DepotCommand>()
        .expect("Unable to start local socket"),
    }
  }

  pub fn spawn_stream<T, P, Pvd>(
    &mut self, provider: Pvd, addr: SocketAddr,
    shutdown_handle: ShutdownHandle, name: String,
  ) -> io::Result<()>
  where
    T: StreamTransport,
    P: StreamProtocol<T>,
    Pvd: StreamProvider<Transport = T, Protocol = P>,
  {
    self.pods.push(Pod::spawn_stream(
      provider,
      addr,
      shutdown_handle,
      name,
      self.current_id,
    )?);
    self.current_id += 1;
    Ok(())
  }

  pub fn spawn_datagram<T, P, Pvd>(
    &mut self, provider: Pvd, addr: SocketAddr,
    shutdown_handle: ShutdownHandle, name: String,
  ) -> io::Result<()>
  where
    T: DatagramTransport,
    P: DatagramProtocol<T>,
    Pvd: DatagramProvider<Transport = T, Protocol = P>,
  {
    self.pods.push(Pod::spawn_datagram(
      provider,
      addr,
      shutdown_handle,
      name,
      self.current_id,
    )?);
    self.current_id += 1;
    Ok(())
  }

  pub fn remove_pod_by_name<S: AsRef<String>>(&mut self, name: S) {
    self.pods.retain(|ele| {
      if ele.name() == name.as_ref() {
        ele.abort();
        false
      } else {
        true
      }
    });
  }

  pub fn remove_pod_by_id(&mut self, id: usize) {
    self.pods.retain(|ele| {
      if ele.id() == id {
        ele.abort();
        false
      } else {
        true
      }
    });
  }

  /// Waits for the root token to be cancelled and then joins every pod.
  ///
  /// Cancelling the root token — through [`shutdown`](Self::shutdown) or
  /// through a handle obtained from [`shutdown_handle`](Self::shutdown_handle)
  /// — stops every pod at once. Each pod is joined afterwards, so this
  /// resolves only once every pod task has ended and the host may flush
  /// the event bus and the logger without losing records.
  pub async fn start(self) {
    self.shutdown_handle.wait().await;

    for pod in self.pods {
      pod.shutdown().await;
    }
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

  pub fn shutdown(&self) {
    self.ipc.abort();
    self.shutdown_handle.shutdown();
  }
}
