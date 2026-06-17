use crate::{HandlingKind, Level, record::Record};

pub trait Flow: Send + 'static {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind>;

  fn flush(&mut self) -> Result<(), HandlingKind>;

  fn can_log(&self, record: &Record) -> bool {
    self.name() == record.target.unwrap_or(self.name())
      && self.level() <= record.level
  }

  fn level(&self) -> Level;

  fn name(&self) -> &'static str;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    Ok(record)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    Ok(())
  }

  fn can_log(&self, _record: &Record) -> bool {
    false
  }

  fn level(&self) -> Level {
    Level::Fatal
  }

  fn name(&self) -> &'static str {
    "Identity"
  }
}

pub struct Stack<F: Flow, S: Flow> {
  pub inner: F,
  pub next: S,
}

impl<F: Flow, S: Flow> Flow for Stack<F, S> {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    if self.inner.can_log(&record) && self.next.can_log(&record) {
      self.inner.println(self.next.println(record)?)
    } else if self.inner.can_log(&record) && !(self.next.can_log(&record)) {
      self.inner.println(record)
    } else if !(self.inner.can_log(&record)) && self.next.can_log(&record) {
      self.next.println(record)
    } else {
      Ok(record)
    }
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    self.next.flush()?;
    self.inner.flush()
  }

  fn can_log(&self, record: &Record) -> bool {
    self.inner.can_log(record) | self.next.can_log(record)
  }

  fn level(&self) -> Level {
    self.inner.level()
  }

  fn name(&self) -> &'static str {
    self.inner.name()
  }
}
