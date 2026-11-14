use std::{mem::take, time::Instant};

use crate::{TimerId, datagram::close::CloseMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerOp {
  Create(Instant),
  Remove,
}

pub struct EffectBuf {
  flush: bool,
  read_pause: Option<bool>,
  close: Option<CloseMode>,
  timers: Vec<(TimerId, TimerOp)>,
}

impl Default for EffectBuf {
  fn default() -> Self {
    Self::new()
  }
}

impl EffectBuf {
  pub fn new() -> Self {
    Self { flush: false, read_pause: None, close: None, timers: Vec::new() }
  }

  pub fn flush(&mut self) {
    self.flush = true;
  }

  pub fn pause_read(&mut self) {
    self.read_pause = Some(true);
  }

  pub fn resume_read(&mut self) {
    self.read_pause = Some(false);
  }

  pub fn close(&mut self, mode: CloseMode) {
    if let Some(legacy_mode) = self.close {
      match (legacy_mode, mode) {
        (CloseMode::Abort, _) | (_, CloseMode::Abort) => {
          self.close = Some(CloseMode::Abort)
        },
        _ => self.close = Some(CloseMode::Graceful),
      }
    } else {
      self.close = Some(mode);
    }
  }

  pub fn arm_timer(&mut self, id: TimerId, at: Instant) {
    self.timers.push((id, TimerOp::Create(at)));
  }

  pub fn cancel_timer(&mut self, id: TimerId) {
    self.timers.push((id, TimerOp::Remove));
  }

  pub fn take_timer_effect(&mut self) -> Vec<(TimerId, TimerOp)> {
    take(&mut self.timers)
  }

  pub fn take_flush_effect(&mut self) -> bool {
    take(&mut self.flush)
  }

  pub fn take_reader_effect(&mut self) -> Option<bool> {
    take(&mut self.read_pause)
  }

  pub fn take_close_effect(&mut self) -> Option<CloseMode> {
    take(&mut self.close)
  }
}
