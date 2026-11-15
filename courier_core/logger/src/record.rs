use std::borrow::Cow;

use crate::Level;

/// A single log record carrying all metadata and content.
///
/// Records are created by the logging macros (`info!`, `warn!`, etc.) or
/// directly through [`Record::new`]. The `content` is the **raw message
/// payload**; the configured [`Format`](crate::Format) renders the record —
/// metadata *and* payload — into the complete output line on the worker
/// thread.
///
/// The timestamp is deliberately **not** part of the record: it is assigned by
/// the logger's worker thread when the record is dequeued, which keeps a clock
/// read off the logging call site.
///
/// # Fields
///
/// - `level` — severity level of the record.
/// - `target` — optional flow name for targeted routing.
/// - `namespace` — identifier for the source module or component.
/// - `content` — the log message payload. Literal messages stay borrowed, so
///   they do not allocate.
///
/// # Examples
///
/// ```rust
/// use logger::{Level, Record};
///
/// let record = Record::new(Level::Info, "my_ns", "hello");
/// assert_eq!(record.level, Level::Info);
/// assert_eq!(&*record.content, "hello");
/// assert!(record.target.is_none());
/// ```
pub struct Record {
  pub level: Level,
  pub target: Option<&'static str>,
  pub namespace: &'static str,
  pub content: Cow<'static, str>,
}

impl Record {
  /// Creates a new record that every flow may accept.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Level, Record};
  ///
  /// let record = Record::new(Level::Warn, "my_ns", format!("x = {}", 42));
  /// assert_eq!(&*record.content, "x = 42");
  /// ```
  pub fn new(
    level: Level, namespace: &'static str,
    content: impl Into<Cow<'static, str>>,
  ) -> Self {
    Self { level, target: None, namespace, content: content.into() }
  }

  /// Creates a new record routed only to the flow named `target`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Level, Record};
  ///
  /// let record =
  ///   Record::with_target(Level::Error, "my_ns", "console", "boom");
  /// assert_eq!(record.target, Some("console"));
  /// ```
  pub fn with_target(
    level: Level, namespace: &'static str, target: &'static str,
    content: impl Into<Cow<'static, str>>,
  ) -> Self {
    Self { level, target: Some(target), namespace, content: content.into() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_sets_fields_correctly() {
    let record = Record::new(Level::Info, "my_namespace", "hello");

    assert_eq!(record.level, Level::Info);
    assert_eq!(record.namespace, "my_namespace");
    assert_eq!(record.target, None);
    assert_eq!(&*record.content, "hello");
  }

  #[test]
  fn literal_content_stays_borrowed() {
    let record = Record::new(Level::Info, "ns", "literal");

    assert!(matches!(record.content, Cow::Borrowed(_)));
  }

  #[test]
  fn formatted_content_is_owned() {
    let record = Record::new(Level::Info, "ns", format!("value = {}", 1));

    assert!(matches!(record.content, Cow::Owned(_)));
    assert_eq!(&*record.content, "value = 1");
  }

  #[test]
  fn with_target_sets_target_field() {
    let record = Record::with_target(Level::Warn, "ns", "target_ns", "msg");

    assert_eq!(record.level, Level::Warn);
    assert_eq!(record.namespace, "ns");
    assert_eq!(record.target, Some("target_ns"));
    assert_eq!(&*record.content, "msg");
  }
}
