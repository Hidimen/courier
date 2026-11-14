use std::{net::SocketAddr, time::Instant};

use crate::{
  Payload, TimerId,
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

  /// Write bytes into `buffer`.
  ///
  /// # Returns
  /// `Ok(())` means all data is written.
  /// `Err((SocketAddr, Payload))` means data is refused due to `len` is
  /// greater than `budget`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
  ///
  /// use protocol::{Payload, datagram::{Context, Output}};
  ///
  /// let mut output = Output::new();
  /// let mut context = Context::new(
  ///   SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
  ///   &mut output,
  /// );
  /// context.write_bytes("1.1.1.1:8080".parse::<SocketAddr>().unwrap(), Payload::from_static(b"hello")).unwrap();
  /// ```
  pub fn write_bytes(
    &mut self, peer: SocketAddr, payload: Payload,
  ) -> Result<(), (SocketAddr, Payload)> {
    self.out.write_bytes(peer, payload)
  }

  pub fn flush(&mut self) {
    self.out.flush();
  }

  pub fn close(&mut self, mode: CloseMode) {
    self.out.close(mode);
  }

  pub fn pause_read(&mut self) {
    self.out.pause_read();
  }

  pub fn resume_read(&mut self) {
    self.out.resume_read();
  }

  pub fn arm_timer(&mut self, id: TimerId, at: Instant) {
    self.out.arm_timer(id, at);
  }

  pub fn cancel_timer(&mut self, id: TimerId) {
    self.out.cancel_timer(id);
  }
}
