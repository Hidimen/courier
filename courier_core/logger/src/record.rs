use bytes::Bytes;
use chrono::Utc;

use crate::Level;

/// A single log record carrying all metadata and content.
///
/// Records are created through the logger's public methods (e.g.
/// [`Logger::info`](crate::Logger::info)) or the convenience macros
/// (`info!`, `warn!`, etc.). The [`DefaultFormatter`](crate::builder::DefaultFormatter)
/// consumes records to produce the final output format.
///
/// # Fields
///
/// - `timestamp` — Unix timestamp in seconds when the record was created.
/// - `level` — severity level of the record.
/// - `target` — optional flow name for targeted routing.
/// - `namespace` — identifier for the source module or component.
/// - `content` — the log message payload.
pub struct Record {
  pub timestamp: i64,
  pub level: Level,
  pub target: Option<&'static str>,
  pub namespace: &'static str,
  pub content: Bytes,
}

impl Record {
  /// Creates a new record from an owned `String`.
  ///
  /// The timestamp is set to the current UTC time.
  pub fn new(content: String, level: Level, namespace: &'static str) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      target: None,
      namespace,
      content: Bytes::from(content),
    }
  }

  /// Creates a new record from a `&'static str` without allocating.
  pub fn new_from_static(
    content: &'static str, level: Level, namespace: &'static str,
  ) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      target: None,
      namespace,
      content: Bytes::from_static(content.as_bytes()),
    }
  }

  /// Creates a new record from an owned `String` with a target for routing.
  pub fn new_with_target(
    content: String, level: Level, namespace: &'static str,
    target: &'static str,
  ) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      target: Some(target),
      namespace,
      content: Bytes::from(content),
    }
  }

  /// Creates a new record from a `&'static str` with a target for routing,
  /// without allocating.
  pub fn new_from_static_with_target(
    content: &'static str, level: Level, namespace: &'static str,
    target: &'static str,
  ) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      target: Some(target),
      namespace,
      content: Bytes::from_static(content.as_bytes()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Utc;

  #[test]
  fn new_sets_fields_correctly() {
    let before = Utc::now().timestamp();
    let record = Record::new("hello".into(), Level::Info, "my_namespace");
    let after = Utc::now().timestamp();

    assert_eq!(record.level, Level::Info);
    assert_eq!(record.namespace, "my_namespace");
    assert_eq!(record.target, None);
    assert_eq!(&record.content[..], b"hello");
    assert!(record.timestamp >= before);
    assert!(record.timestamp <= after);
  }

  #[test]
  fn new_from_static_sets_fields_correctly() {
    let record =
      Record::new_from_static("static msg", Level::Error, "static_ns");

    assert_eq!(record.level, Level::Error);
    assert_eq!(record.namespace, "static_ns");
    assert_eq!(record.target, None);
    assert_eq!(&record.content[..], b"static msg");
  }

  #[test]
  fn new_with_target_sets_target_field() {
    let record =
      Record::new_with_target("msg".into(), Level::Warn, "ns", "target_ns");

    assert_eq!(record.level, Level::Warn);
    assert_eq!(record.target, Some("target_ns"));
    assert_eq!(record.namespace, "ns");
    assert_eq!(&record.content[..], b"msg");
  }

  #[test]
  fn new_from_static_with_target_sets_all_fields() {
    let record = Record::new_from_static_with_target(
      "static",
      Level::Fatal,
      "namespace",
      "target",
    );

    assert_eq!(record.level, Level::Fatal);
    assert_eq!(record.namespace, "namespace");
    assert_eq!(record.target, Some("target"));
    assert_eq!(&record.content[..], b"static");
  }

  #[test]
  fn timestamp_is_recent() {
    let now = Utc::now().timestamp();
    let record = Record::new("test".into(), Level::Info, "ns");

    let diff = (now - record.timestamp).abs();
    assert!(diff <= 2, "timestamp should be within 2 seconds of now");
  }
}
