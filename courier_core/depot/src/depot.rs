use std::{
  io,
  net::SocketAddr,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU32, Ordering},
  },
};

use ipc::LocalSocket;
use network::transport::{DatagramTransport, StreamTransport};
use protocol::{DatagramProtocol, StreamProtocol};
use sync::handle::ShutdownHandle;

use crate::{DatagramProvider, Pod, StreamProvider, control::DepotCommand};

static INSTANCE: OnceLock<Arc<Depot>> = OnceLock::new();

pub struct Depot {
  pods: Mutex<Vec<Pod>>,
  current_id: AtomicU32,
  ipc: LocalSocket,
  shutdown_handle: ShutdownHandle,
}

impl Depot {
  /// Creates an instance with no pods and a root shutdown token that
  /// carries `shutdown_timeout` as its grace period.
  pub fn new(shutdown_timeout: usize) -> Self {
    let shutdown_handle = ShutdownHandle::new(shutdown_timeout);

    Self {
      pods: Mutex::new(Vec::new()),
      current_id: AtomicU32::new(0),
      ipc: LocalSocket::new::<DepotCommand>(shutdown_handle.child())
        .expect("Unable to start local socket"),
      shutdown_handle,
    }
  }

  pub fn install(self) -> Arc<Self> {
    let this = Arc::new(self);
    if INSTANCE.set(this.clone()).is_err() {
      panic!("Depot is installed twice");
    }

    this
  }

  pub fn get_instance() -> Arc<Self> {
    if let Some(this) = INSTANCE.get() {
      this.clone()
    } else {
      panic!("Depot is not installed yet");
    }
  }

  pub fn spawn_stream<T, P, Pvd>(
    &self, provider: Pvd, addr: SocketAddr, name: String,
  ) -> io::Result<()>
  where
    T: StreamTransport,
    P: StreamProtocol<T>,
    Pvd: StreamProvider<Transport = T, Protocol = P>,
  {
    self.pods.lock().unwrap().push(Pod::spawn_stream(
      provider,
      addr,
      self.shutdown_handle.child(),
      name,
      self.current_id.load(Ordering::Acquire),
    )?);
    self.current_id.fetch_add(1, Ordering::Release);
    Ok(())
  }

  pub fn spawn_datagram<T, P, Pvd>(
    &mut self, provider: Pvd, addr: SocketAddr, name: String,
  ) -> io::Result<()>
  where
    T: DatagramTransport,
    P: DatagramProtocol<T>,
    Pvd: DatagramProvider<Transport = T, Protocol = P>,
  {
    self.pods.lock().unwrap().push(Pod::spawn_datagram(
      provider,
      addr,
      self.shutdown_handle.child(),
      name,
      self.current_id.load(Ordering::Acquire),
    )?);
    self.current_id.fetch_add(1, Ordering::Release);
    Ok(())
  }

  pub fn remove_pod_by_name<S: AsRef<String>>(&mut self, name: S) {
    self.pods.lock().unwrap().retain(|ele| {
      if ele.name() == name.as_ref() {
        ele.abort();
        false
      } else {
        true
      }
    });
  }

  pub fn remove_pod_by_id(&mut self, id: u32) {
    self.pods.lock().unwrap().retain(|ele| {
      if ele.id() == id {
        ele.abort();
        false
      } else {
        true
      }
    });
  }

  pub fn list_pods_info(&self) -> Vec<(u32, String)> {
    self
      .pods
      .lock()
      .unwrap()
      .iter()
      .map(|e| (e.id(), e.name().to_string()))
      .collect()
  }

  /// Waits for the root token to be cancelled and then joins every pod.
  ///
  /// Cancelling the root token — through [`shutdown`](Self::shutdown) or
  /// through a handle obtained from [`shutdown_handle`](Self::shutdown_handle)
  /// — stops every pod at once. Each pod is joined afterwards, so this
  /// resolves only once every pod task has ended and the host may flush
  /// the event bus and the logger without losing records.
  pub async fn start(&self) {
    self.shutdown_handle.wait().await;

    for pod in self.pods.lock().unwrap().iter() {
      pod.shutdown();
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
