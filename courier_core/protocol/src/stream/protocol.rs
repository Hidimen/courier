use std::task::{Poll, Waker};

use crate::{
  Protocol,
  stream::{Context, Input},
};

pub trait StreamProtocol: Protocol {
  fn step(
    &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
  ) -> Result<(), Self::Error>;

  fn poll(
    &mut self, ctx: &mut Context<'_>, waker: &Waker,
  ) -> Poll<Result<(), Self::Error>> {
    let _ = (ctx, waker);
    Poll::Pending
  }
}
