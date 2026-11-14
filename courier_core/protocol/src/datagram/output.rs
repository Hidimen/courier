use std::{mem::take, net::SocketAddr, time::Instant};

use crate::{
  Payload, TimerId,
  datagram::{
    close::CloseMode,
    effect::{EffectBuf, TimerOp},
  },
};

pub struct Output {
  parts: Vec<(SocketAddr, Payload)>,
  effects: EffectBuf,
  budget: usize,
  remaining: usize,
}

impl Output {
  pub const DEFAULT_BUDGET: usize = 16 * 1024;

  pub fn new() -> Self {
    Self {
      parts: Vec::new(),
      effects: EffectBuf::new(),
      budget: Self::DEFAULT_BUDGET,
      remaining: Self::DEFAULT_BUDGET,
    }
  }

  /// Write bytes into `buffer`.
  ///
  /// # Returns
  /// `None` means all data is written.
  /// `Some((SocketAddr, Payload))` means data is refused due to `len` is
  /// greater than `budget`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
  ///
  /// use protocol::{Payload, datagram::Output};
  ///
  /// let mut output = Output::new();
  /// let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
  /// assert!(output.write_bytes(peer, Payload::from_static(b"hello")).is_ok());
  /// ```
  pub fn write_bytes(
    &mut self, peer: SocketAddr, bytes: Payload,
  ) -> Result<(), (SocketAddr, Payload)> {
    if bytes.len() > self.remaining {
      Err((peer, bytes))
    } else {
      let len = bytes.len();
      if !bytes.is_empty() {
        self.parts.push((peer, bytes));
      }
      self.remaining -= len;
      Ok(())
    }
  }

  pub fn get_effects(&mut self) -> &mut EffectBuf {
    &mut self.effects
  }

  /// Takes every buffered datagram together with the total byte length.
  ///
  /// The budget is restored, so the next step starts with a full budget.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
  ///
  /// use protocol::{Payload, datagram::Output};
  ///
  /// let mut output = Output::new();
  /// let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
  /// assert!(output.write_bytes(peer, Payload::from_static(b"hello")).is_ok());
  ///
  /// let (parts, len) = output.take_bytes();
  /// assert_eq!(len, 5);
  /// assert_eq!(parts.len(), 1);
  /// ```
  pub fn take_bytes(&mut self) -> (Vec<(SocketAddr, Payload)>, usize) {
    let len = self.budget - self.remaining;
    self.remaining = self.budget;
    (take(&mut self.parts), len)
  }

  pub fn flush(&mut self) {
    self.effects.flush();
  }

  pub fn pause_read(&mut self) {
    self.effects.pause_read();
  }

  pub fn resume_read(&mut self) {
    self.effects.resume_read();
  }

  pub fn close(&mut self, mode: CloseMode) {
    self.effects.close(mode);
  }

  pub fn arm_timer(&mut self, id: TimerId, at: Instant) {
    self.effects.arm_timer(id, at);
  }

  pub fn cancel_timer(&mut self, id: TimerId) {
    self.effects.cancel_timer(id);
  }

  pub fn take_timer_effect(&mut self) -> Vec<(TimerId, TimerOp)> {
    self.effects.take_timer_effect()
  }

  pub fn take_flush_effect(&mut self) -> bool {
    self.effects.take_flush_effect()
  }

  pub fn take_reader_effect(&mut self) -> Option<bool> {
    self.effects.take_reader_effect()
  }

  pub fn take_close_effect(&mut self) -> Option<CloseMode> {
    self.effects.take_close_effect()
  }
}

impl Default for Output {
  fn default() -> Self {
    Self::new()
  }
}
