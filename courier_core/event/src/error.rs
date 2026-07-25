use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegisterError {
  #[error("Event already exists")]
  EventAlreadyExists,
}

#[derive(Debug, Error)]
pub enum UnregisterError {
  #[error("Event not found")]
  EventNotFound,
}

#[derive(Debug, Error)]
pub enum EmitError {
  #[error("Event channel is full")]
  Full,
  #[error("Event channel is closed")]
  Closed,
}

#[derive(Debug, Error)]
pub enum SubscribeError {
  #[error("Event not found")]
  EventNotFound,
}

#[derive(Debug, Error)]
pub enum UnsubscribeError {
  #[error("Event not found")]
  EventNotFound,
  #[error("Subscriber not found")]
  SubscriberNotFound,
}
