use std::error::Error;

#[derive(Debug, thiserror::Error)]
#[error("Listener error: {0}")]
pub struct ListenerError(Box<dyn Error + Send + Sync + 'static>);

impl ListenerError {
  pub fn new<E>(error: E) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self(Box::new(error))
  }

  pub fn message<S: Into<String>>(msg: S) -> Self {
    Self(msg.into().into())
  }
}
