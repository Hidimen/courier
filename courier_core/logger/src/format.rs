use std::fmt::{self, Write as _};

use crate::{Level, Record};

/// Renders the complete textual form of a log record.
///
/// A `Format` has **full control** over the output: it decides what to emit,
/// in which order, whether to include the raw payload, how to escape it,
/// whether to use ANSI colours or JSON, whether to emit several lines, and
/// whether/how the output is terminated. There is no framework-imposed
/// "header + payload" layout — whatever you write into `out` is the final
/// text that sinks receive.
///
/// Formatting runs on the logger's worker thread, once per record, into a
/// reusable buffer that the framework clears between records. Implementations
/// should therefore write directly into `out` (which is empty on entry) and
/// avoid allocating scratch buffers of their own — steady-state formatting
/// then allocates nothing.
///
/// `timestamp` is the pre-rendered, second-resolution timestamp string for
/// `record.timestamp` (e.g. `2026-01-02 03:04:05`). The worker renders it at
/// most once per wall-clock second and reuses the cached string for every
/// record in that second; prefer it over formatting the time yourself.
///
/// Implement this trait to define a custom log format. Every closure of type
/// `Fn(&Record, &str, &mut String) -> fmt::Result + Send + 'static`
/// automatically implements `Format`, so quick custom formats can be passed
/// inline to [`Builder::format`](crate::Builder::format).
///
/// For line-oriented sinks (console, files) a formatter should terminate its
/// output with `\n`.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write as _;
/// use logger::{Format, Level, Record};
///
/// struct MyFormatter;
///
/// impl Format for MyFormatter {
///     fn format(
///         &self,
///         record: &Record,
///         _timestamp: &str,
///         out: &mut String,
///     ) -> std::fmt::Result {
///         // The payload is just one of many things you may write.
///         write!(
///             out,
///             "[MY-APP] [{}] {}\n",
///             record.namespace,
///             String::from_utf8_lossy(&record.content)
///         )
///     }
/// }
///
/// let record = Record::new("hello".into(), Level::Info, "app");
/// let mut line = String::new();
/// MyFormatter
///     .format(&record, "2026-01-02 03:04:05", &mut line)
///     .unwrap();
/// assert_eq!(line, "[MY-APP] [app] hello\n");
/// ```
pub trait Format: Send + 'static {
  /// Renders the complete textual representation of `record` into `out`.
  ///
  /// `out` is empty when this method is called and is reused across records;
  /// only ever append to it. `timestamp` is a pre-rendered, cached
  /// `YYYY-MM-DD HH:MM:SS` string for `record.timestamp`.
  fn format(
    &self, record: &Record, timestamp: &str, out: &mut String,
  ) -> fmt::Result;
}

impl<T> Format for T
where
  T: Fn(&Record, &str, &mut String) -> fmt::Result + Send + 'static,
{
  fn format(
    &self, record: &Record, timestamp: &str, out: &mut String,
  ) -> fmt::Result {
    (self)(record, timestamp, out)
  }
}

/// A default formatter that produces a `[timestamp][LEVEL][NAMESPACE] message`
/// layout.
///
/// # Example
///
/// ```rust
/// use logger::{DefaultFormatter, Format, Level, Record};
///
/// let record = Record::new("hello".into(), Level::Info, "my_ns");
/// let mut line = String::new();
/// DefaultFormatter
///     .format(&record, "2026-01-02 03:04:05", &mut line)
///     .unwrap();
///
/// assert_eq!(line, "[2026-01-02 03:04:05][INFO][my_ns] hello\n");
/// ```
pub struct DefaultFormatter;

impl Format for DefaultFormatter {
  fn format(
    &self, record: &Record, timestamp: &str, out: &mut String,
  ) -> fmt::Result {
    write!(
      out,
      "[{}][{}][{}] {}\n",
      timestamp,
      record.level,
      record.namespace,
      String::from_utf8_lossy(&record.content)
    )
  }
}

/// A pretty formatter that produces a `timestamp LEVEL NAMESPACE message`
/// layout with colours.
///
/// # Example
///
/// ```rust
/// use logger::{PrettyFormatter, Format, Level, Record};
///
/// let record = Record::new("hello".into(), Level::Info, "my_ns");
/// let mut line = String::new();
/// PrettyFormatter
///     .format(&record, "2026-01-02 03:04:05", &mut line)
///     .unwrap();
///
/// assert_eq!(
///     line,
///     "2026-01-02 03:04:05 \x1b[92m\x1b[1mINFO\x1b[0m my_ns hello\n"
/// );
/// ```
pub struct PrettyFormatter;

impl Format for PrettyFormatter {
  fn format(
    &self, record: &Record, timestamp: &str, out: &mut String,
  ) -> fmt::Result {
    let (open, close): (&str, &str) = match record.level {
      Level::Trace => ("\x1b[95m\x1b[1m", "\x1b[0m"),
      Level::Debug => ("\x1b[94m\x1b[1m", "\x1b[0m"),
      Level::Info => ("\x1b[92m\x1b[1m", "\x1b[0m"),
      Level::Warn => ("\x1b[93m\x1b[1m", "\x1b[0m"),
      Level::Error => ("\x1b[31m\x1b[1m", "\x1b[0m"),
      Level::Fatal => ("\x1b[97m\x1b[41m\x1b[1m", "\x1b[0m"),
    };
    write!(
      out,
      "{} {}{}{} {} {}\n",
      timestamp,
      open,
      record.level,
      close,
      record.namespace,
      String::from_utf8_lossy(&record.content)
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Level;

  fn render<F: Format>(format: F, record: &Record) -> String {
    let mut line = String::new();
    format.format(record, "2026-01-02 03:04:05", &mut line).unwrap();
    line
  }

  #[test]
  fn closure_format_controls_full_line() {
    let format = |record: &Record, _timestamp: &str, out: &mut String| {
      // Full control: payload first, then metadata, then a newline.
      write!(
        out,
        "{}|{}|{}\n",
        String::from_utf8_lossy(&record.content),
        record.namespace,
        record.level
      )
    };

    let record = Record::new("hello".into(), Level::Info, "test");
    let line = render(format, &record);

    assert_eq!(line, "hello|test|INFO\n");
  }

  #[test]
  fn noop_format_writes_empty_output() {
    let format = |_record: &Record, _timestamp: &str, _out: &mut String| Ok(());

    let record = Record::new("unchanged".into(), Level::Debug, "test");
    let line = render(format, &record);

    assert!(line.is_empty());
    assert_eq!(&record.content[..], b"unchanged");
    assert_eq!(record.level, Level::Debug);
    assert_eq!(record.namespace, "test");
  }

  #[test]
  fn default_formatter_layout() {
    let record = Record::new("hello".into(), Level::Info, "my_ns");
    let line = render(DefaultFormatter, &record);

    assert_eq!(line, "[2026-01-02 03:04:05][INFO][my_ns] hello\n");
    assert!(line.starts_with('['));
    assert!(line.contains("][INFO][my_ns] hello"));
  }

  #[test]
  fn pretty_formatter_layout() {
    let record = Record::new("hello".into(), Level::Info, "my_ns");
    let line = render(PrettyFormatter, &record);

    assert_eq!(
      line,
      "2026-01-02 03:04:05 \x1b[92m\x1b[1mINFO\x1b[0m my_ns hello\n"
    );
  }

  #[test]
  fn pretty_formatter_per_level_colours() {
    let cases: &[(Level, &str)] = &[
      (Level::Trace, "\x1b[95m\x1b[1mTRACE\x1b[0m"),
      (Level::Debug, "\x1b[94m\x1b[1mDEBUG\x1b[0m"),
      (Level::Info, "\x1b[92m\x1b[1mINFO\x1b[0m"),
      (Level::Warn, "\x1b[93m\x1b[1mWARN\x1b[0m"),
      (Level::Error, "\x1b[31m\x1b[1mERROR\x1b[0m"),
      (Level::Fatal, "\x1b[97m\x1b[41m\x1b[1mFATAL\x1b[0m"),
    ];

    for (level, expected) in cases {
      let record = Record::new("".into(), *level, "ns");
      let line = render(PrettyFormatter, &record);
      assert!(
        line.contains(expected),
        "level {level:?} should render colour tag {expected:?}, got {line:?}"
      );
    }
  }
}
