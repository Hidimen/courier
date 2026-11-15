use std::io::{Write, stdout};

use crate::{HandlingKind, Level, Record, flow::Flow};

/// A flow that writes log records to standard output.
///
/// Each record arrives as a **fully rendered line** — the formatter owns the
/// layout, including the trailing newline — and is written verbatim via a
/// locked stdout handle, so output is not interleaved with other writers.
///
/// # Example
///
/// ```rust,no_run
/// use logger::{Builder, Level, Record};
/// use logger::flows::ConsoleFlow;
///
/// let logger = Builder::new()
///   .capacity(128)
///   .add_flow(ConsoleFlow::new(Level::Info))
///   .build_local();
///
/// logger.log(Record::new(Level::Info, "demo", "Hello console!"));
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
  fn println(
    &mut self, _record: &Record, line: &str,
  ) -> Result<(), HandlingKind> {
    let mut handle = stdout().lock();
    handle.write_all(line.as_bytes()).map_err(|_| HandlingKind::Ignore)?;
    Ok(())
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    stdout().flush().map_err(|_| HandlingKind::Ignore)
  }

  fn level(&self) -> Level {
    self.level
  }

  fn name(&self) -> &'static str {
    "courier:console"
  }
}
