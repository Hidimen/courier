use marquage::{Generate, Parse};

use crate::Level;

/// Default number of records the logger channel buffers when no capacity is
/// configured.
///
/// A capacity of zero would turn the channel into a rendezvous channel, where
/// a non-blocking send only succeeds while the worker is parked in `recv`,
/// so back-to-back records would be silently dropped.
pub(crate) const DEFAULT_CAPACITY: usize = 1024;

/// Represents the configuration of a [`Logger`](crate::Logger).
#[derive(Generate, Parse, Debug)]
pub struct Config {
  /// Lowest level the console flow accepts.
  #[marquage(default)]
  pub level: Level,
  /// Number of records the internal channel buffers.
  #[marquage(default = 1024)]
  pub capacity: usize,
  /// Layout used to render every record.
  #[marquage(default)]
  pub format: FormatConfig,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      level: Level::default(),
      capacity: DEFAULT_CAPACITY,
      format: FormatConfig::default(),
    }
  }
}

/// Represents the supported log layouts.
#[derive(Debug, Generate, Parse, Default)]
#[marquage(rename = "lowercase")]
pub enum FormatConfig {
  /// The `[timestamp][LEVEL][namespace] message` layout.
  #[default]
  Default,
  /// The coloured `timestamp LEVEL namespace message` layout.
  Pretty,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_capacity_is_non_zero() {
    assert_eq!(Config::default().capacity, DEFAULT_CAPACITY);
    assert!(Config::default().capacity > 0);
  }

  #[test]
  fn default_level_is_info() {
    assert_eq!(Config::default().level, Level::Info);
  }
}
