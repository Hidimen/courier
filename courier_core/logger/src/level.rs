use std::fmt::Display;

/// Represents the severity level of a log record.
///
/// The levels are ordered from least to most severe: `Trace < Debug < Info
/// < Warn < Error < Fatal`. This ordering is derived via `#[derive(Ord)]`,
/// matching the variant declaration order.
///
/// Each flow declares its minimum accepted level; records below that level
/// are silently discarded.
///
/// # Example
///
/// ```rust,no_run
/// use logger::Level;
///
/// assert!(Level::Trace < Level::Info);
/// assert_eq!(Level::Error.to_string(), "ERROR");
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Level {
  /// The finest-grained, most verbose log level.
  Trace,
  /// Diagnostic information useful for debugging.
  Debug,
  /// General informational messages.
  Info,
  /// Potential issues that do not prevent the application from running.
  Warn,
  /// Errors that should be investigated.
  Error,
  /// Critical errors that may cause the application to terminate.
  Fatal,
}

impl Display for Level {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Trace => "TRACE",
        Self::Debug => "DEBUG",
        Self::Info => "INFO",
        Self::Warn => "WARN",
        Self::Error => "ERROR",
        Self::Fatal => "FATAL",
      }
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn display_outputs() {
    assert_eq!(Level::Trace.to_string(), "TRACE");
    assert_eq!(Level::Debug.to_string(), "DEBUG");
    assert_eq!(Level::Info.to_string(), "INFO");
    assert_eq!(Level::Warn.to_string(), "WARN");
    assert_eq!(Level::Error.to_string(), "ERROR");
    assert_eq!(Level::Fatal.to_string(), "FATAL");
  }

  #[test]
  fn ordering() {
    assert!(Level::Trace < Level::Debug);
    assert!(Level::Debug < Level::Info);
    assert!(Level::Info < Level::Warn);
    assert!(Level::Warn < Level::Error);
    assert!(Level::Error < Level::Fatal);
  }

  #[test]
  fn equality() {
    assert_eq!(Level::Trace, Level::Trace);
    assert_ne!(Level::Trace, Level::Debug);
  }

  #[test]
  fn clone_copy() {
    let level = Level::Info;
    let copied = level;
    assert_eq!(level, copied);
    let cloned = level;
    assert_eq!(level, cloned);
  }
}
