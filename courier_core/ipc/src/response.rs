use std::error::Error;

use crate::Framed;

pub trait Response: Sized {
  type Error: Error + Send + Sync + 'static;

  fn from_framed(framed: Framed) -> Result<Self, Self::Error>;

  fn handle(&mut self) -> Result<(), Self::Error>;
}
