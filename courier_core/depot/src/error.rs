use thiserror::Error;
use tokio::task::JoinError;

/// Errors that can occur while running a [`Depot`](crate::Depot).
#[derive(Debug, Error)]
pub enum DepotError {
  /// An I/O error from the underlying transport.
  #[error("Io error occurred: {0}")]
  Io(
    #[source]
    #[from]
    std::io::Error,
  ),

  /// An execution error emitted by protocol or middlewares.
  ///
  /// **Note**: It contains nothing and serves as a placeholder.
  #[error("Execution error occurred")]
  Execution,

  /// A spawned task terminated unexpectedly.
  #[error("Task terminated: {0}")]
  TaskTerminated(JoinError),
}

/// Errors that can occur while building a [`Depot`](crate::Depot) via
/// [`DepotBuilder`](crate::DepotBuilder).
///
/// Each variant indicates a required field that was not set before
/// [`build`](crate::DepotBuilder::build) was called.
#[derive(Debug, Error)]
pub enum DepotBuildError {
  /// Transport was not set.
  #[error("transport is required but was not set")]
  MissingTransport,

  /// Protocol was not set.
  #[error("protocol is required but was not set")]
  MissingProtocol,

  /// Pipeline was not set.
  #[error("pipeline is required but was not set")]
  MissingPipeline,

  /// Logger was not set.
  #[error("logger is required but was not set")]
  MissingLogger,
}
