use std::ops::Deref;

use tokio::task::JoinHandle;
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

#[derive(Clone)]
pub struct ShutdownHandle(CancellationToken);

impl Default for ShutdownHandle {
  fn default() -> Self {
    Self::new()
  }
}

impl ShutdownHandle {
  pub fn new() -> Self {
    Self(CancellationToken::new())
  }

  pub fn wait(&self) -> WaitForCancellationFuture<'_> {
    self.0.cancelled()
  }

  pub fn child(&self) -> Self {
    Self(self.0.child_token())
  }

  pub fn shutdown(&self) {
    self.0.cancel();
  }

  pub fn is_shutdown(&self) -> bool {
    self.0.is_cancelled()
  }
}

pub struct RunningHandle(pub JoinHandle<()>);

impl Deref for RunningHandle {
  type Target = JoinHandle<()>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
