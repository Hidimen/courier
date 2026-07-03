use std::{
  fs::{File, OpenOptions},
  io::{BufWriter, Write},
  path::Path,
};

use crate::{HandlingKind, Level, Record, flow::Flow};

/// A flow that writes log records to a file.
///
/// Records are appended to the file behind a [`BufWriter`], so writes are
/// buffered for efficiency. Call [`flush`](Flow::flush) or drop the
/// [`Logger`](crate::Logger) to ensure all data reaches disk.
///
/// # Panics
///
/// The constructor panics if the file cannot be opened for appending.
///
/// # Example
///
/// ```rust,no_run
/// use logger::{Builder, Level, Logger};
/// use logger::flows::FileFlow;
///
/// let logger = Logger::new(128, FileFlow::new(Level::Info, "app.log"), |r| r);
/// logger.info("Hello file!".into(), "demo");
/// drop(logger); // flushes and closes
/// ```
pub struct FileFlow {
  level: Level,
  buf: BufWriter<File>,
}

impl FileFlow {
  /// Creates a new `FileFlow` that appends to the given `path`.
  ///
  /// The file is created if it does not exist. Records at or above `level`
  /// are accepted.
  ///
  /// # Panics
  ///
  /// Panics if the file cannot be opened.
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
