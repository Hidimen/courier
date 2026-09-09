use std::ops::Deref;

use tokio::task::JoinHandle;
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

#[derive(Clone)]
pub struct ShutdownHandle(CancellationToken, usize);

impl ShutdownHandle {
  pub fn new(timeout: usize) -> Self {
    Self(CancellationToken::new(), timeout)
  }

  pub fn wait(&self) -> WaitForCancellationFuture<'_> {
    self.0.cancelled()
  }

  pub fn child(&self) -> Self {
    Self(self.0.child_token(), self.1)
  }

  pub fn shutdown(&self) {
    self.0.cancel();
  }

  pub fn is_shutdown(&self) -> bool {
    self.0.is_cancelled()
  }

  pub fn timeout(&self) -> usize {
    self.1
  }
}

pub struct RunningHandle(pub JoinHandle<()>);

impl Deref for RunningHandle {
  type Target = JoinHandle<()>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
