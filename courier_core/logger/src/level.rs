use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Level {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
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
