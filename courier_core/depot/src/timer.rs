//! Per-connection timer bookkeeping used by the stream driver.
//!
//! This module is an implementation detail of the driver: it is public only
//! so benchmarks and tests can drive it directly.

use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  time::Instant,
};

use protocol::TimerId;

/// The timers of a single connection.
///
/// `live` is the authoritative `id -> deadline` map, while `due` may contain
/// stale entries left behind by re-arming or cancelling a timer. Stale
/// entries are discarded lazily when they reach the top of the heap, and the
/// heap is rebuilt once it grows beyond twice the number of live timers.
pub struct TimerSet {
  due: BinaryHeap<Reverse<(Instant, TimerId)>>,
  live: HashMap<TimerId, Instant>,
}

impl Default for TimerSet {
  fn default() -> Self {
    Self::new()
  }
}

impl TimerSet {
  /// Creates an empty timer set.
  pub fn new() -> Self {
    Self { due: BinaryHeap::new(), live: HashMap::new() }
  }

  /// Arms `id` at `at`, replacing any deadline previously set for `id`.
  pub fn arm(&mut self, id: TimerId, at: Instant) {
    self.live.insert(id, at);
    self.due.push(Reverse((at, id)));
    self.compact();
  }

  /// Cancels `id`; cancelling an unknown or already fired timer is a no-op.
  pub fn cancel(&mut self, id: TimerId) {
    if self.live.remove(&id).is_some() {
      self.compact();
    }
  }

  /// Returns the earliest live timer, dropping stale heap entries on the way.
  pub fn next_due(&mut self) -> Option<(Instant, TimerId)> {
    while let Some(&Reverse((at, id))) = self.due.peek() {
      if self.live.get(&id) == Some(&at) {
        return Some((at, id));
      }
      self.due.pop();
    }

    None
  }

  /// Removes `id` after delivery, so one arm fires exactly once.
  pub fn take(&mut self, id: TimerId) {
    self.live.remove(&id);
  }

  /// Removes every timer.
  pub fn clear(&mut self) {
    self.live.clear();
    self.due.clear();
  }

  fn compact(&mut self) {
    if self.due.len() > self.live.len() * 2 + 8 {
      let entries = self.live.iter().map(|(&id, &at)| Reverse((at, id)));
      self.due = BinaryHeap::from_iter(entries);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::time::{Duration, Instant};

  use protocol::TimerId;

  use super::TimerSet;

  #[test]
  fn replaces_existing_deadline() {
    let mut set = TimerSet::new();
    let now = Instant::now();

    set.arm(TimerId(1), now + Duration::from_millis(20));
    set.arm(TimerId(1), now + Duration::from_millis(10));

    assert_eq!(
      set.next_due(),
      Some((now + Duration::from_millis(10), TimerId(1)))
    );

    set.take(TimerId(1));
    assert_eq!(set.next_due(), None);
  }

  #[test]
  fn cancel_is_a_no_op_for_unknown_ids() {
    let mut set = TimerSet::new();

    set.cancel(TimerId(7));
    assert_eq!(set.next_due(), None);

    set.arm(TimerId(7), Instant::now());
    set.cancel(TimerId(7));
    assert_eq!(set.next_due(), None);
  }
}
