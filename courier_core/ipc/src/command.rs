use std::error::Error;

use bytes::Bytes;

use crate::Framed;

pub trait Command: Send + Sync + Sized + 'static {
  type Error: Error + Send + Sync + 'static;

  fn from_framed(framed: Framed) -> Result<Self, Self::Error>;

  fn handle(
    &self,
  ) -> impl Future<Output = Result<(u16, Bytes), Self::Error>> + Send;
}
