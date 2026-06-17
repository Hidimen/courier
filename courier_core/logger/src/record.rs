use bytes::Bytes;
use chrono::Utc;

use crate::Level;

pub struct Record {
  pub timestamp: i64,
  pub level: Level,
  pub target: Option<&'static str>,
  pub namespace: &'static str,
  pub content: Bytes,
}

impl Record {
  pub fn new(content: String, level: Level, namespace: &'static str) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      target: None,
      namespace,
      content: Bytes::from(content),
    }
  }

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
