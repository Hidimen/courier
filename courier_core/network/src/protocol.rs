use async_trait::async_trait;

use crate::{Frame, transport::Transport};

#[async_trait]
pub trait Protocol<T: Transport>: Send + Sync + Sized + 'static {
  type Message: Sync + Send + 'static;

  fn name() -> &'static str;

  fn version() -> &'static str;
}
