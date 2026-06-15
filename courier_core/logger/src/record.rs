use bytes::Bytes;
use chrono::Utc;

use crate::Level;

pub struct Record {
  pub timestamp: i64,
  pub level: Level,
  pub content: Bytes,
}

impl Record {
  pub fn new(content: String, level: Level) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      content: Bytes::from(content),
    }
  }

  pub fn new_from_static(content: &'static str, level: Level) -> Self {
    Self {
      timestamp: Utc::now().timestamp(),
      level,
      content: Bytes::from_static(content.as_bytes()),
    }
  }
}
