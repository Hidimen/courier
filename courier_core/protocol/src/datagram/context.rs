use std::{net::SocketAddr, time::Instant};

use crate::{
  TimerId,
  datagram::{Output, close::CloseMode},
};

pub struct Context<'a> {
  now: Instant,
  peer: SocketAddr,
  out: &'a mut Output,
}

impl<'a> Context<'a> {
  pub fn new(peer: SocketAddr, out: &'a mut Output) -> Self {
    Self { now: Instant::now(), peer, out }
  }

  pub fn now(&self) -> Instant {
    self.now
  }

  pub fn peer(&self) -> SocketAddr {
    self.peer
  }

  pub fn flush(&mut self) {
    self.out.effects.flush();
  }

  pub fn close(&mut self, mode: CloseMode) {
    self.out.effects.close(mode);
  }

  pub fn pause_read(&mut self) {
    self.out.effects.pause_read();
  }

  pub fn resume_read(&mut self) {
    self.out.effects.resume_read();
  }

  pub fn arm_timer(&mut self, id: TimerId, at: Instant) {
    self.out.effects.arm_timer(id, at);
  }

  pub fn cancel_timer(&mut self, id: TimerId) {
    self.out.effects.cancel_timer(id);
  }
}
