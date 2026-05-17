use std::fmt::Debug;

pub trait ProtocolError: std::error::Error + Send + Sync + 'static {
  fn severity(&self) -> ErrorSeverity;
}

#[derive(Debug)]
pub enum ErrorSeverity {
  Recoverable,

  Ignorable,

  Fatal,

  Panic,
}
