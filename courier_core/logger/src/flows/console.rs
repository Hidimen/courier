use std::io::{BufWriter, Write, stdout};

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
    if self.level > record.level {
      return Ok(record);
    }

    let mut handle = stdout().lock();
    handle.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
    handle.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;
    Ok(record)
  }

  fn print_batch(
    &mut self, records: Vec<Record>,
  ) -> Result<Vec<Record>, HandlingKind> {
    let mut buf = BufWriter::new(stdout());
    for record in records.iter() {
      if self.level > record.level {
        continue;
      }
      buf.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
      buf.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;
    }

    buf.flush().map_err(|_| HandlingKind::Ignore)?;

    Ok(records)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    stdout().flush().map_err(|_| HandlingKind::Ignore)
  }
}
