use std::io::{Write, stdout};

use crate::{HandlingKind, Level, Record, flow::Flow};

/// A flow that writes log records to standard output.
///
/// Each record is written as a single line to `stdout` (via a locked
/// handle, so output is not interleaved with other writers).
///
/// # Example
///
/// ```rust
/// use logger::{Builder, Level, Logger};
/// use logger::flows::ConsoleFlow;
///
/// let logger = Logger::new(128, ConsoleFlow::new(Level::Info), |r| r);
/// logger.info("Hello console!".into(), "demo");
/// drop(logger);
/// ```
pub struct ConsoleFlow {
  level: Level,
}

impl ConsoleFlow {
  /// Creates a new `ConsoleFlow` that accepts records at or above the given
  /// `level`.
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
