use std::time::Instant;

use crate::{TimerId, datagram::close::CloseMode};

#[derive(Debug, Clone, Copy)]
pub enum Effect {
  Flush,
  Close(CloseMode),

  PauseRead,
  ResumeRead,

  ArmTimer { id: TimerId, at: Instant },
  CancelTimer(TimerId),

  NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimerOp {
  Create(Instant),
  Remove,
}

pub struct EffectBuf {
  flush: bool,
  read_pause: bool,
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
    Self { flush: false, read_pause: false, close: None, timers: Vec::new() }
  }

  pub fn flush(&mut self) {
    self.flush = true;
  }

  pub fn pause_read(&mut self) {
    self.read_pause = true;
  }

  pub fn resume_read(&mut self) {
    self.read_pause = false;
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

  pub fn take_timer_effect(
    &mut self,
  ) -> impl std::iter::Iterator<Item = Effect> {
    let list = std::mem::take(&mut self.timers);
    list.into_iter().map(|(id, op)| match op {
      TimerOp::Create(at) => Effect::ArmTimer { id, at },
      TimerOp::Remove => Effect::CancelTimer(id),
    })
  }

  pub fn take_flush_effect(&mut self) -> Effect {
    if self.flush {
      self.flush = false;
      Effect::Flush
    } else {
      Effect::NoOp
    }
  }

  pub fn take_reader_effect(&mut self) -> Effect {
    if self.read_pause { Effect::PauseRead } else { Effect::ResumeRead }
  }

  pub fn take_close_effect(&mut self) -> Effect {
    if let Some(mode) = self.close { Effect::Close(mode) } else { Effect::NoOp }
  }
}
