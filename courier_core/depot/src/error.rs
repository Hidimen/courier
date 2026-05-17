use std::fmt::Display;

use protocol::error::ProtocolError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum DepotError<E: ProtocolError> {
  #[error("Io error occurred: {0}")]
  Io(
    #[from]
    #[source]
    std::io::Error,
  ),

  #[error("Protocol error occurred: {0}")]
  Protocol(#[from] ProtocolErrorWrapper<E>),

  #[error("Task terminated: {0}")]
  TaskTerminated(JoinError),
}

#[derive(Debug)]
pub struct ProtocolErrorWrapper<E: ProtocolError>(pub E);

impl<E: ProtocolError> std::error::Error for ProtocolErrorWrapper<E> {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.0)
  }
}

impl<E: ProtocolError> Display for ProtocolErrorWrapper<E> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl<E: ProtocolError> From<E> for ProtocolErrorWrapper<E> {
  fn from(value: E) -> Self {
    Self(value)
  }
}
