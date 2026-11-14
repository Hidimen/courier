use std::task::{Poll, Waker};

use crate::{
  Protocol,
  stream::{Context, Input},
};

pub trait StreamProtocol: Protocol {
  fn step(
    &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
  ) -> Result<(), Self::Error>;

  /// Polls the protocol's own work.
  ///
  /// `Pending` means work is outstanding and `waker` has been registered;
  /// `Ready` means the protocol has nothing left to do until it receives new
  /// input. A protocol without its own event sources keeps the default.
  fn poll(
    &mut self, ctx: &mut Context<'_>, waker: &Waker,
  ) -> Poll<Result<(), Self::Error>> {
    let _ = (ctx, waker);
    Poll::Ready(Ok(()))
  }
}
