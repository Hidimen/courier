use std::fmt::Display;

use chrono::Utc;

use crate::Level;

pub struct Message {
  pub timestamp: i64,
  pub level: Level,
  pub content: Content,
}

impl Message {
  pub fn new(content: String, level: Level) -> Self {
    Self { timestamp: Utc::now().timestamp(), level, content: Content::Dynamic(content) }
  }

  pub fn new_from_static(content: &'static str, level: Level) -> Self {
    Self { timestamp: Utc::now().timestamp(), level, content: Content::Static(content) }
  }
}

pub enum Content {
  Static(&'static str),
  Dynamic(String),
}

impl Content {
  pub fn get(&self) -> &str {
    match self {
      Self::Dynamic(s) => s,
      Self::Static(s) => s,
    }
  }
}

impl Display for Content {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Dynamic(s) => write!(f, "{}", s),
      Self::Static(s) => write!(f, "{}", s),
    }
  }
}
