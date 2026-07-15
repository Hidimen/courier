use chrono::DateTime;

use crate::Record;

/// Formats log records before they are dispatched to flows.
///
/// Implement this trait to define a custom log format. Every closure of type
/// `Fn(Record) -> Record + Send + 'static` automatically implements `Format`,
/// so quick custom formatters can be passed inline to
/// [`Builder::format`](crate::Builder::format).
///
/// # Example
///
/// ```rust
/// use logger::{Format, Level, Record};
///
/// struct MyFormatter;
///
/// impl Format for MyFormatter {
///     fn format(&self, mut record: Record) -> Record {
///         record.content = format!("[MY-APP] {}", String::from_utf8_lossy(&record.content)).into();
///         record
///     }
/// }
///
/// let record = Record::new("hello".into(), Level::Info, "app");
/// let result = MyFormatter.format(record);
/// assert!(std::str::from_utf8(&result.content).unwrap().starts_with("[MY-APP]"));
/// ```
pub trait Format: Send + 'static {
  /// Transforms the given `record` and returns the result.
  fn format(&self, record: Record) -> Record;
}

impl<T> Format for T
where
  T: Fn(Record) -> Record + Send + 'static,
{
  fn format(&self, record: Record) -> Record {
    (self)(record)
  }
}

/// A default formatter that produces a `[timestamp][LEVEL] message` layout.
///
/// # Example
///
/// ```rust
/// use logger::{DefaultFormatter, Format, Level, Record};
///
/// let formatter = DefaultFormatter;
/// let record = Record::new("hello".into(), Level::Info, "my_ns");
/// let result = formatter.format(record);
///
/// let output = String::from_utf8_lossy(&result.content);
/// assert!(output.starts_with('['));
/// assert!(output.contains("][INFO] hello"));
/// ```
pub struct DefaultFormatter;

impl Format for DefaultFormatter {
  fn format(&self, mut record: Record) -> Record {
    let raw = unsafe { String::from_utf8_unchecked(record.content.into()) };
    let new = format!(
      "[{}][{}][{}] {}",
      DateTime::from_timestamp(record.timestamp, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S"),
      record.level,
      record.namespace,
      raw
    );
    record.content = new.into();
    record
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Level;
  use bytes::Bytes;

  #[test]
  fn closure_format_transforms_record() {
    let format = |mut r: Record| {
      r.content = Bytes::from(format!(
        "[PREFIX] {}",
        String::from_utf8_lossy(&r.content)
      ));
      r
    };

    let record = Record::new("hello".into(), Level::Info, "test");
    let result = format.format(record);

    assert_eq!(&result.content[..], b"[PREFIX] hello");
  }

  #[test]
  fn noop_format_returns_same_record() {
    let format = |r: Record| r;

    let record = Record::new("unchanged".into(), Level::Debug, "test");
    let result = format.format(record);

    assert_eq!(&result.content[..], b"unchanged");
    assert_eq!(result.level, Level::Debug);
    assert_eq!(result.namespace, "test");
  }

  #[test]
  fn format_can_change_level() {
    let format = |mut r: Record| {
      r.level = Level::Error;
      r
    };

    let record = Record::new("msg".into(), Level::Info, "test");
    let result = format.format(record);

    assert_eq!(result.level, Level::Error);
  }
}
