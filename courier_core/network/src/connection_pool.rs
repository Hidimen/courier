use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{AcquireError, Notify, OwnedSemaphorePermit, Semaphore};

// ---------------------------------------------------------------------------
// Internal state
// -------------------------------------------------------------------

/// Per-key metadata tracked by the pool.
struct Entry {
  /// Number of connections currently registered under this key.
  count: usize,
  /// Timestamp of the most recent [`ConnGuard::touch`] call for
  /// this key.
  last_active: Instant,
}

impl Entry {
  fn new() -> Self {
    Self { count: 1, last_active: Instant::now() }
  }
}

/// Shared state between [`ConnectionPool`] and its [`ConnGuard`]s.
struct Inner<K> {
  /// Active-entry map.
  entries: Mutex<HashMap<K, Entry>>,
}

// ---------------------------------------------------------------------------
// ShutdownRx
// ---------------------------------------------------------------------------

/// A receiver for the pool's graceful-shutdown signal.
///
/// Obtain one via [`ConnectionPool::shutdown_rx`] and `.await` it
/// inside a `tokio::select!` branch — it completes once when
/// [`ConnectionPool::shutdown`] is called.
///
/// # Examples
///
/// ```ignore
/// let shutdown_rx = pool.shutdown_rx();
/// tokio::select! {
///     result = stream.read(&mut buf) => { /* handle data */ }
///     _ = shutdown_rx.wait() => {
///         // Graceful shutdown — finish current work and return.
///         return;
///     }
/// }
/// ```
pub struct ShutdownRx {
  /// Whether shutdown has been signalled.
  flag: Arc<AtomicBool>,
  /// The Notify that wakes waiters when shutdown is called.
  notify: Arc<Notify>,
}

impl ShutdownRx {
  /// Waits until the pool's [`shutdown`](ConnectionPool::shutdown)
  /// method is called.
  ///
  /// If shutdown has already been signalled, this returns
  /// immediately.
  pub async fn wait(&self) {
    if self.flag.load(Ordering::Acquire) {
      return;
    }
    self.notify.notified().await;
  }
}

impl Clone for ShutdownRx {
  fn clone(&self) -> Self {
    Self { flag: Arc::clone(&self.flag), notify: Arc::clone(&self.notify) }
  }
}

impl Debug for ShutdownRx {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ShutdownRx").finish_non_exhaustive()
  }
}

// ---------------------------------------------------------------------------
// ConnGuard
// ---------------------------------------------------------------------------

/// A RAII guard representing a registered connection.
///
/// **Holding this guard keeps the connection registered in the pool.**
/// When the guard is dropped (including via task cancellation), the
/// connection is automatically unregistered and its capacity slot is
/// returned to the pool.
///
/// Use [`touch`](Self::touch) after each successful request-response
/// cycle to keep the connection from appearing in
/// [`reap_idle`](ConnectionPool::reap_idle) results.
#[must_use = "the connection is unregistered when this guard is dropped"]
pub struct ConnGuard<K: Hash + Eq> {
  /// The key this connection was registered under.  Taken out on drop.
  key: Option<K>,
  /// Holds one capacity slot from the pool's semaphore.
  _permit: OwnedSemaphorePermit,
  /// Back-link to the pool so we can unregister on drop.
  inner: Arc<Inner<K>>,
}

impl<K> ConnGuard<K>
where
  K: Hash + Eq,
{
  /// Returns the key this connection is registered under.
  pub fn key(&self) -> &K {
    self.key.as_ref().expect("ConnGuard key already consumed on drop")
  }

  /// Updates the last-active timestamp for this connection to now.
  ///
  /// Call this after successfully processing a request so that
  /// [`ConnectionPool::reap_idle`] considers this connection fresh.
  pub fn touch(&self) {
    let Some(ref key) = self.key else {
      return;
    };
    let mut entries = self.inner.entries.lock().unwrap();
    if let Some(entry) = entries.get_mut(key) {
      entry.last_active = Instant::now();
    }
  }
}

impl<K> Debug for ConnGuard<K>
where
  K: Hash + Eq + Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ConnGuard").field("key", &self.key).finish_non_exhaustive()
  }
}

impl<K> Drop for ConnGuard<K>
where
  K: Hash + Eq,
{
  fn drop(&mut self) {
    let Some(key) = self.key.take() else {
      return;
    };
    let mut entries = self.inner.entries.lock().unwrap();
    if let Some(entry) = entries.get_mut(&key) {
      entry.count -= 1;
      if entry.count == 0 {
        entries.remove(&key);
      }
    }
    // `_permit` is dropped here → semaphore slot released.
  }
}

// ---------------------------------------------------------------------------
// ConnectionPool
// ---------------------------------------------------------------------------

/// A server-side connection pool for managing active keep-alive
/// connections.
///
/// # Role
///
/// Unlike a client-side pool where idle connections are checked out
/// and returned, a server-side pool tracks **active** connections.
/// Each connection stays in its own tokio task, looping on
/// `read().await` — the tokio reactor (epoll / kqueue / IOCP) wakes
/// the task when data arrives on the socket.  The pool provides:
///
/// - **Capacity limit** — rejects or queues connections above
///   `max_connections`.
/// - **Connection registry** — tracks how many connections are
///   active per key (typically a peer [`SocketAddr`]).
/// - **Shutdown broadcast** — wakes all connection tasks at once for
///   graceful shutdown via [`ShutdownRx`].
/// - **Idle detection** — reports keys that have been inactive
///   longer than a threshold so the caller can evict them.
///
/// # Architecture sketch
///
/// ```ignore
/// use courier_network::connection_pool::ConnectionPool;
///
/// # async fn example() {
/// let pool = ConnectionPool::<std::net::SocketAddr>::new(10_000);
///
/// // --- Accept loop ---
/// // let guard = pool.try_register(peer_addr)
/// //     .expect("pool at capacity");
/// // let shutdown = pool.shutdown_rx();
/// //
/// // tokio::spawn(async move {
/// //     loop {
/// //         tokio::select! {
/// //             result = stream.read(&mut buf) => { /* handle */ }
/// //             _ = shutdown.wait() => { break; }  // graceful
/// //             _ = tokio::time::sleep(IDLE) => { break; } // idle
/// //         }
/// //         guard.touch();  // keep-alive heartbeat
/// //     }
/// //     // guard drops here → auto-unregister
/// // });
/// # }
/// ```
///
/// # Examples
///
/// ```ignore
/// use courier_network::connection_pool::ConnectionPool;
/// use std::net::SocketAddr;
///
/// let pool = ConnectionPool::<SocketAddr>::new(10_000);
/// assert_eq!(pool.available(), 10_000);
/// ```
pub struct ConnectionPool<K> {
  /// Per-key connection registry.
  inner: Arc<Inner<K>>,
  /// Capacity semaphore — one permit per connection.
  semaphore: Arc<Semaphore>,
  /// Whether [`shutdown`](Self::shutdown) has been called.
  shutdown_flag: Arc<AtomicBool>,
  /// Shutdown broadcast — `notify_waiters()` wakes every
  /// [`ShutdownRx::wait`] caller that is currently waiting.
  shutdown_notify: Arc<Notify>,
}

impl<K> ConnectionPool<K>
where
  K: Hash + Eq + Clone,
{
  /// Creates a new pool allowing at most `max_connections`
  /// concurrent connections.
  ///
  /// # Panics
  ///
  /// Panics if `max_connections` is zero.
  pub fn new(max_connections: usize) -> Self {
    assert!(max_connections > 0, "max_connections must be > 0");
    Self {
      inner: Arc::new(Inner { entries: Mutex::new(HashMap::new()) }),
      semaphore: Arc::new(Semaphore::new(max_connections)),
      shutdown_flag: Arc::new(AtomicBool::new(false)),
      shutdown_notify: Arc::new(Notify::new()),
    }
  }

  // ------------------------------------------------------------------
  // Registration
  // ------------------------------------------------------------------

  /// Attempts to register a new connection without waiting.
  ///
  /// Returns `Some(ConnGuard)` if the pool has spare capacity, or
  /// `None` when it is full.  The returned guard keeps the
  /// connection registered — drop it to release the slot.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// match pool.try_register(peer_addr) {
  ///     Some(guard) => { /* spawn connection task */ }
  ///     None => { /* respond 503 — pool full */ }
  /// }
  /// ```
  pub fn try_register(&self, key: K) -> Option<ConnGuard<K>> {
    let permit = self.semaphore.clone().try_acquire_owned().ok()?;

    {
      let mut entries = self.inner.entries.lock().unwrap();
      match entries.get_mut(&key) {
        Some(entry) => {
          entry.count += 1;
          entry.last_active = Instant::now();
        },
        None => {
          entries.insert(key.clone(), Entry::new());
        },
      }
    }

    Some(ConnGuard {
      key: Some(key),
      _permit: permit,
      inner: Arc::clone(&self.inner),
    })
  }

  /// Registers a new connection, waiting asynchronously for a
  /// capacity slot if the pool is full.
  ///
  /// This is useful when you would rather queue than reject.
  ///
  /// # Errors
  ///
  /// Returns [`AcquireError::Closed`] if the semaphore has been
  /// closed.  The pool never closes its semaphore under normal
  /// operation, so this is unlikely.
  pub async fn register(&self, key: K) -> Result<ConnGuard<K>, AcquireError> {
    let permit = self.semaphore.clone().acquire_owned().await?;

    {
      let mut entries = self.inner.entries.lock().unwrap();
      match entries.get_mut(&key) {
        Some(entry) => {
          entry.count += 1;
          entry.last_active = Instant::now();
        },
        None => {
          entries.insert(key.clone(), Entry::new());
        },
      }
    }

    Ok(ConnGuard {
      key: Some(key),
      _permit: permit,
      inner: Arc::clone(&self.inner),
    })
  }

  // ------------------------------------------------------------------
  // Shutdown
  // ------------------------------------------------------------------

  /// Signals all connection tasks to begin graceful shutdown.
  ///
  /// Every pending [`ShutdownRx::wait`] call resolves immediately,
  /// and any [`ShutdownRx`] obtained after this call also resolves
  /// immediately.
  pub fn shutdown(&self) {
    self.shutdown_flag.store(true, Ordering::Release);
    self.shutdown_notify.notify_waiters();
  }

  /// Returns a [`ShutdownRx`] that resolves when
  /// [`shutdown`](Self::shutdown) is called.
  ///
  /// Each connection task should obtain one receiver and select on
  /// `.wait()` to react to the shutdown signal.
  pub fn shutdown_rx(&self) -> ShutdownRx {
    ShutdownRx {
      flag: Arc::clone(&self.shutdown_flag),
      notify: Arc::clone(&self.shutdown_notify),
    }
  }

  // ------------------------------------------------------------------
  // Idle detection
  // ------------------------------------------------------------------

  /// Returns the keys of connections whose last
  /// [`touch`](ConnGuard::touch) was more than `max_idle` ago.
  ///
  /// **This method only reports** — it is the caller's
  /// responsibility to actually close those connections (e.g. by
  /// dropping the guard or signaling the task).  The entries remain
  /// in the registry until the guards are dropped.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// let idle_keys = pool.reap_idle(Duration::from_secs(60));
  /// for key in &idle_keys {
  ///     // Signal the connection task(s) for this key to close.
  /// }
  /// ```
  pub fn reap_idle(&self, max_idle: Duration) -> Vec<K> {
    let entries = self.inner.entries.lock().unwrap();
    let now = Instant::now();

    entries
      .iter()
      .filter(|(_, entry)| now - entry.last_active >= max_idle)
      .map(|(key, _)| key.clone())
      .collect()
  }

  // ------------------------------------------------------------------
  // Introspection
  // ------------------------------------------------------------------

  /// Returns the number of capacity slots still available.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// let pool = ConnectionPool::<u32>::new(100);
  /// assert_eq!(pool.available(), 100);
  /// ```
  pub fn available(&self) -> usize {
    self.semaphore.available_permits()
  }

  /// Returns the total number of actively registered connections.
  ///
  /// This is `max_connections - available()`.
  pub fn active_count(&self) -> usize {
    // The semaphore has been initialised with max_connections
    // permits, but we don't store that value.  We can compute it
    // indirectly, or we can track it separately.
    //
    // We track it separately to avoid overflow corner-cases.
    self.inner.entries.lock().unwrap().values().map(|e| e.count).sum()
  }

  /// Returns the number of active connections registered under
  /// `key`.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// assert_eq!(pool.len_for_key(&peer_addr), 3);
  /// ```
  pub fn len_for_key(&self, key: &K) -> usize {
    self.inner.entries.lock().unwrap().get(key).map(|e| e.count).unwrap_or(0)
  }

  /// Returns `true` when no connections are registered.
  pub fn is_empty(&self) -> bool {
    self.inner.entries.lock().unwrap().is_empty()
  }
}

// ---------------------------------------------------------------------------
// Trait impls
// ---------------------------------------------------------------------------

impl<K> Debug for ConnectionPool<K>
where
  K: Hash + Eq + Clone + Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ConnectionPool")
      .field("available", &self.available())
      .field("active_count", &self.active_count())
      .finish_non_exhaustive()
  }
}

impl<K> Clone for ConnectionPool<K> {
  /// Clones the pool handle.
  ///
  /// All clones share the same backing storage — registering a
  /// connection on one clone is visible to all others.
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
      semaphore: Arc::clone(&self.semaphore),
      shutdown_flag: Arc::clone(&self.shutdown_flag),
      shutdown_notify: Arc::clone(&self.shutdown_notify),
    }
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  // ------------------------------------------------------------------
  // Registration
  // ------------------------------------------------------------------

  #[tokio::test]
  async fn try_register_within_capacity() {
    let pool = ConnectionPool::<u32>::new(2);
    assert_eq!(pool.available(), 2);

    let g1 = pool.try_register(1);
    assert!(g1.is_some());
    assert_eq!(pool.available(), 1);
    assert_eq!(pool.active_count(), 1);
  }

  #[tokio::test]
  async fn try_register_at_capacity_returns_none() {
    let pool = ConnectionPool::<u32>::new(1);
    let _g1 = pool.try_register(1).unwrap();

    let g2 = pool.try_register(2);
    assert!(g2.is_none());
  }

  #[tokio::test]
  async fn guard_drop_releases_capacity() {
    let pool = ConnectionPool::<u32>::new(1);
    assert_eq!(pool.available(), 1);

    let g1 = pool.try_register(1).unwrap();
    assert_eq!(pool.available(), 0);

    drop(g1);
    assert_eq!(pool.available(), 1);
    assert_eq!(pool.active_count(), 0);

    // Should succeed now.
    assert!(pool.try_register(2).is_some());
  }

  #[tokio::test]
  async fn register_waits_for_capacity() {
    let pool = ConnectionPool::<u32>::new(1);
    let _g1 = pool.try_register(1).unwrap();

    // Spawn a task that will block on register, then drop g1.
    let pool2 = pool.clone();
    let handle = tokio::spawn(async move { pool2.register(2).await.unwrap() });

    // Give the spawned task a moment to block.
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Release the held slot.
    drop(_g1);

    let g2 = handle.await.unwrap();
    assert_eq!(g2.key(), &2);
  }

  #[tokio::test]
  async fn same_key_increments_count() {
    let pool = ConnectionPool::<u32>::new(10);

    let g1 = pool.try_register(1).unwrap();
    let g2 = pool.try_register(1).unwrap();
    assert_eq!(pool.len_for_key(&1), 2);

    drop(g1);
    assert_eq!(pool.len_for_key(&1), 1);

    drop(g2);
    assert_eq!(pool.len_for_key(&1), 0);
  }

  // ------------------------------------------------------------------
  // Shutdown
  // ------------------------------------------------------------------

  #[tokio::test]
  async fn shutdown_wakes_all_receivers() {
    let pool = ConnectionPool::<u32>::new(10);

    let rx1 = pool.shutdown_rx();
    let rx2 = pool.shutdown_rx();

    let (tx1, tx2) = (
      tokio::spawn(async move { rx1.wait().await }),
      tokio::spawn(async move { rx2.wait().await }),
    );

    // Neither should finish before shutdown.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(!tx1.is_finished());
    assert!(!tx2.is_finished());

    pool.shutdown();

    tx1.await.unwrap();
    tx2.await.unwrap();
  }

  #[tokio::test]
  async fn shutdown_rx_resolves_immediately_after_shutdown() {
    let pool = ConnectionPool::<u32>::new(10);
    pool.shutdown();

    // Must return immediately, not block.
    pool.shutdown_rx().wait().await;
  }

  // ------------------------------------------------------------------
  // Idle detection
  // ------------------------------------------------------------------

  #[tokio::test]
  async fn reap_idle_zero_timeout_finds_all() {
    let pool = ConnectionPool::<u32>::new(10);
    let _g1 = pool.try_register(1).unwrap();
    let _g2 = pool.try_register(2).unwrap();

    // Zero timeout — everything is idle (Duration::ZERO trivially
    // matches any timestamp).
    let idle = pool.reap_idle(Duration::ZERO);
    assert!(idle.contains(&1));
    assert!(idle.contains(&2));
  }

  #[tokio::test]
  async fn reap_idle_large_timeout_finds_none() {
    let pool = ConnectionPool::<u32>::new(10);
    let _g1 = pool.try_register(1).unwrap();
    let _g2 = pool.try_register(2).unwrap();

    // Fresh entries survive a large timeout.
    let idle = pool.reap_idle(Duration::from_secs(3600));
    assert!(idle.is_empty());
  }

  #[tokio::test]
  async fn reap_idle_preserves_registry_entries() {
    let pool = ConnectionPool::<u32>::new(10);
    let _g = pool.try_register(1).unwrap();

    // reap_idle only reports — it must not remove entries.
    let _idle = pool.reap_idle(Duration::ZERO);
    assert_eq!(pool.len_for_key(&1), 1);

    let _idle = pool.reap_idle(Duration::from_secs(3600));
    assert_eq!(pool.len_for_key(&1), 1);
  }

  // ------------------------------------------------------------------
  // Introspection
  // ------------------------------------------------------------------

  #[tokio::test]
  async fn active_count_and_available() {
    let pool = ConnectionPool::<u32>::new(5);
    assert_eq!(pool.available(), 5);
    assert_eq!(pool.active_count(), 0);

    let _g1 = pool.try_register(1).unwrap();
    let _g2 = pool.try_register(2).unwrap();

    assert_eq!(pool.available(), 3);
    assert_eq!(pool.active_count(), 2);
  }

  #[tokio::test]
  async fn is_empty() {
    let pool = ConnectionPool::<u32>::new(5);
    assert!(pool.is_empty());

    let g = pool.try_register(1).unwrap();
    assert!(!pool.is_empty());

    drop(g);
    assert!(pool.is_empty());
  }

  // ------------------------------------------------------------------
  // Clone
  // ------------------------------------------------------------------

  #[tokio::test]
  async fn clone_shares_state() {
    let pool_a = ConnectionPool::<u32>::new(5);
    let pool_b = pool_a.clone();

    let _g = pool_a.try_register(1).unwrap();
    assert_eq!(pool_b.available(), 4);
    assert_eq!(pool_b.active_count(), 1);
  }
}
