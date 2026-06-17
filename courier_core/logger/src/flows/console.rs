use std::io::{Write, stdout};

use crate::{HandlingKind, Level, Record, flow::Flow};

pub struct ConsoleFlow {
  level: Level,
}

impl ConsoleFlow {
  pub fn new(level: Level) -> Self {
    Self { level }
  }
}

impl Flow for ConsoleFlow {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    let mut handle = stdout().lock();
    handle.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
    handle.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;
    Ok(record)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    stdout().flush().map_err(|_| HandlingKind::Ignore)
  }

  fn level(&self) -> Level {
    self.level
  }

  fn name(&self) -> &'static str {
    "console"
  }
}
