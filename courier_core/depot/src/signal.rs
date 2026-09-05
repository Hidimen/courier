/// Waits for termination signal.
///
/// This function will block on until termination signal comes in.
///
/// # Platform-specific
/// On unix-like, it waits for `SIGTERM`. On Windows, it waits for `Ctrl+C`.
pub async fn wait_for_termination() {
  #[cfg(unix)]
  {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {},
      _ = term.recv() => {},
    }
  }
  #[cfg(not(unix))]
  {
    let _ = tokio::signal::ctrl_c().await;
  }
}
