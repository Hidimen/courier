use std::{mem::take, time::Instant};

use bytes::BytesMut;

use crate::{
  Payload, TimerId,
  stream::{
    close::CloseMode,
    effect::{EffectBuf, TimerOp},
  },
};

pub struct Output {
  parts: Vec<Payload>,
  copy_buf: BytesMut,
  effects: EffectBuf,
  budget: usize,
  remaining: usize,
}

impl Output {
  pub const DEFAULT_BUDGET: usize = 16 * 1024;

  pub fn new() -> Self {
    Self {
      parts: Vec::new(),
      copy_buf: BytesMut::new(),
      effects: EffectBuf::new(),
      budget: Self::DEFAULT_BUDGET,
      remaining: Self::DEFAULT_BUDGET,
    }
  }

  pub fn write(&mut self, src: &[u8]) -> usize {
    let n = src.len().min(self.remaining);
    self.copy_buf.extend_from_slice(&src[..n]);
    self.remaining -= n;
    n
  }

  /// Writes a whole [`Payload`] into this output.
  ///
  /// The write is atomic: the payload is accepted in full, or it is
  /// returned unchanged when it does not fit in the remaining budget.
  ///
  /// # Errors
  ///
  /// Returns the payload back when `payload.len()` is greater than the
  /// remaining budget.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::{Payload, stream::Output};
  ///
  /// let mut output = Output::new();
  /// output.write_bytes(Payload::from_static(b"hello")).unwrap();
  /// ```
  pub fn write_bytes(&mut self, bytes: Payload) -> Result<(), Payload> {
    if bytes.len() > self.remaining {
      Err(bytes)
    } else {
      let len = bytes.len();
      if !self.copy_buf.is_empty() {
        self.parts.push(Payload::from(self.copy_buf.split().freeze()));
      }
      if !bytes.is_empty() {
        self.parts.push(bytes);
      }
      self.remaining -= len;
      Ok(())
    }
  }

  /// Takes every buffered payload together with the total byte length.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::{Payload, stream::Output};
  ///
  /// let mut output = Output::new();
  /// output.write_bytes(Payload::from_static(b"hello")).unwrap();
  ///
  /// let (parts, len) = output.take_bytes();
  /// assert_eq!(len, 5);
  /// assert_eq!(parts.iter().map(|part| part.len()).sum::<usize>(), 5);
  /// ```
  pub fn take_bytes(&mut self) -> (Vec<Payload>, usize) {
    if self.copy_buf.is_empty() {
      let len = self.budget - self.remaining;
      self.remaining = self.budget;
      (take(&mut self.parts), len)
    } else {
      self.parts.push(Payload::from(self.copy_buf.split().freeze()));
      let len = self.budget - self.remaining;
      self.remaining = self.budget;
      (take(&mut self.parts), len)
    }
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

#[cfg(test)]
mod tests {
  use super::Output;
  use crate::Payload;

  #[test]
  fn write_bytes_never_emits_empty_payloads() {
    let mut output = Output::new();
    output.write_bytes(Payload::from_static(b"hello")).unwrap();

    let (parts, len) = output.take_bytes();
    assert_eq!(len, 5);
    assert!(parts.iter().all(|part| !part.is_empty()));
  }

  #[test]
  fn write_bytes_still_flushes_bytes_from_write() {
    let mut output = Output::new();
    output.write(b"abc");
    output.write_bytes(Payload::from_static(b"def")).unwrap();

    let (parts, len) = output.take_bytes();
    assert_eq!(len, 6);
    assert_eq!(
      parts.iter().flat_map(|part| part.iter().copied()).collect::<Vec<_>>(),
      b"abcdef"
    );
  }
}
