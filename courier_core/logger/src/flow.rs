use crate::{HandlingKind, record::Record};

pub trait Flow: Send + 'static {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind>;

  fn print_batch(
    &mut self, records: Vec<Record>,
  ) -> Result<Vec<Record>, HandlingKind>;

  fn flush(&mut self) -> Result<(), HandlingKind>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    Ok(record)
  }

  fn print_batch(
    &mut self, records: Vec<Record>,
  ) -> Result<Vec<Record>, HandlingKind> {
    Ok(records)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    Ok(())
  }
}

pub struct Stack<F: Flow, S: Flow> {
  pub first: F,
  pub second: S,
}

impl<F: Flow, S: Flow> Flow for Stack<F, S> {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    self.first.println(self.second.println(record)?)
  }

  fn print_batch(
    &mut self, records: Vec<Record>,
  ) -> Result<Vec<Record>, HandlingKind> {
    self.first.print_batch(self.second.print_batch(records)?)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    self.second.flush()?;
    self.first.flush()
  }
}
