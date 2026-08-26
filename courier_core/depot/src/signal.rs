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
