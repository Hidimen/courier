use std::task::{Poll, Waker};

use crate::{
  Protocol,
  datagram::{Context, Input},
};

pub trait DatagramProtocol: Protocol {
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
