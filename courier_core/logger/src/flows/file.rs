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
    self.buf.write_all(&record.content).map_err(|_| HandlingKind::Ignore)?;
    self.buf.write_all(b"\n").map_err(|_| HandlingKind::Ignore)?;

    Ok(record)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    self.buf.flush().map_err(|_| HandlingKind::Ignore)
  }

  fn level(&self) -> Level {
    self.level
  }

  fn name(&self) -> &'static str {
    "file"
  }
}
