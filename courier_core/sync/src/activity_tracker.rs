use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::Notify;

pub struct ActivityTracker {
  active: AtomicUsize,
  zero_notify: Notify,
}

impl ActivityTracker {
  pub fn new() -> Self {
    Self { active: AtomicUsize::new(0), zero_notify: Notify::new() }
  }

  pub fn acquire(&self) {
    self.active.fetch_add(1, Ordering::Relaxed);
  }

  pub fn release(&self) {
    if self.active.fetch_sub(1, Ordering::AcqRel) == 1 {
      self.zero_notify.notify_waiters();
    }
  }

  pub fn count(&self) -> usize {
    self.active.load(Ordering::Acquire)
  }

  pub fn is_zero(&self) -> bool {
    self.count() == 0
  }

  /// Wait for the count to become 0.
  pub async fn wait(&self) {
    loop {
      let notified = self.zero_notify.notified();
      tokio::pin!(notified);
      notified.as_mut().enable();

      if self.is_zero() {
        return;
      }

      notified.await;
    }
  }
}

impl Default for ActivityTracker {
  fn default() -> Self {
    Self::new()
  }
}
