use std::io::{IsTerminal, Write, stdout};

use crate::{HandlingKind, Level, Record, flow::Flow};

/// A flow that writes log records to standard output.
///
/// Each record arrives as a **fully rendered line** — the formatter owns the
/// layout, including the trailing newline — and is written verbatim via a
/// locked stdout handle, so output is not interleaved with other writers.
///
/// Whether the process is attached to a terminal is detected **once** when
/// the flow is created; when `stdout` is not a terminal (e.g. redirected to
/// a file or a pipe) the flow drops records, matching the behaviour of many
/// console sinks.
///
/// # Example
///
/// ```rust
/// use logger::{Logger, Level};
/// use logger::flows::ConsoleFlow;
///
/// let logger = Logger::new(
///   128,
///   ConsoleFlow::new(Level::Info),
///   |_record: &logger::Record, _timestamp: &str, _out: &mut String| Ok(()),
/// );
/// logger.info("Hello console!".into(), "demo");
/// drop(logger);
/// ```
pub struct ConsoleFlow {
  level: Level,
  terminal: bool,
}

impl ConsoleFlow {
  /// Creates a new `ConsoleFlow` that accepts records at or above the given
  /// `level`.
  pub fn new(level: Level) -> Self {
    Self { level, terminal: stdout().is_terminal() }
  }
}

impl Flow for ConsoleFlow {
  fn println(
    &mut self, _record: &Record, line: &str,
  ) -> Result<(), HandlingKind> {
    if !self.terminal {
      return Ok(());
    }

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
    "console"
  }
}
