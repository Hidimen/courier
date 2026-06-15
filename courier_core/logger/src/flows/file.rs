use std::{
  fs::{File, OpenOptions},
  io::{BufWriter, Write},
  path::Path,
};

use crate::{HandlingKind, Level, Record, flow::Flow};

pub struct FileFlow {
  level: Level,
  buf: BufWriter<File>,
}

impl FileFlow {
  pub fn new<P: AsRef<Path>>(level: Level, path: P) -> Self {
    Self {
      level,
      buf: BufWriter::with_capacity(
        8196,
        OpenOptions::new()
          .read(false)
          .truncate(false)
          .create(true)
          .append(true)
          .open(path)
          .expect("Unable to open log files"),
      ),
    }
  }
}

impl Flow for FileFlow {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    if self.level > record.level {
      return Ok(record);
    }

    self.buf.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
    self.buf.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;

    Ok(record)
  }

  fn print_batch(
    &mut self, records: Vec<Record>,
  ) -> Result<Vec<Record>, HandlingKind> {
    for record in records.iter() {
      if self.level > record.level {
        continue;
      }

      self.buf.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
      self.buf.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;
    }

    Ok(records)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    self.buf.flush().map_err(|_| HandlingKind::Ignore)
  }
}
