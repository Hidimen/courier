use std::net::SocketAddr;

use crate::{Payload, datagram::effect::EffectBuf};

pub struct Output {
  pub(crate) parts: Vec<(SocketAddr, Payload)>,
  pub(crate) effects: EffectBuf,
  pub(crate) budget: usize,
}

impl Output {
  pub const DEFAULT_BUDGET: usize = 16 * 1024;

  pub fn new() -> Self {
    Self {
      parts: Vec::new(),
      effects: EffectBuf::new(),
      budget: Self::DEFAULT_BUDGET,
    }
  }

  /// Write bytes into `buffer`.
  ///
  /// # Returns
  /// `None` means all data is written.
  /// `Some(Payload)` means data is refused due to `len` is
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
  /// assert!(output.write_bytes(peer, Payload::from_static(b"hello")).is_none());
  /// ```
  pub fn write_bytes(
    &mut self, peer: SocketAddr, bytes: Payload,
  ) -> Option<Payload> {
    if bytes.len() > self.budget {
      Some(bytes)
    } else {
      self.budget -= bytes.len();
      self.parts.push((peer, bytes));
      None
    }
  }

  pub fn get_effects(&mut self) -> &mut EffectBuf {
    &mut self.effects
  }
}

impl Default for Output {
  fn default() -> Self {
    Self::new()
  }
}
