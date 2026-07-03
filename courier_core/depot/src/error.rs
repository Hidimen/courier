use std::error::Error;

use thiserror::Error;
use tokio::task::JoinError;

/// Errors that can occur while running a [`Depot`](crate::Depot).
///
/// `E` is the error type of the pipeline's middleware chain.
#[derive(Debug, Error)]
pub enum DepotError<E: Error + Send + Sync + 'static> {
  /// An I/O error from the underlying transport.
  #[error("Io error occurred: {0}")]
  Io(#[source] std::io::Error),

  /// An error returned by protocol or middlewares.
  #[error("Middleware error: {0}")]
  Process(#[source] E),

  /// A spawned task terminated unexpectedly.
  #[error("Task terminated: {0}")]
  TaskTerminated(JoinError),
}

impl<E: Error + Send + Sync + 'static> From<std::io::Error> for DepotError<E> {
  fn from(err: std::io::Error) -> Self {
    DepotError::Io(err)
  }
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
