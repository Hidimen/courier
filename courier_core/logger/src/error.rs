/// Represents the outcome of handling a log record by a [`Flow`](crate::flow::Flow).
///
/// When a flow's [`println`](crate::flow::Flow::println) or
/// [`flush`](crate::flow::Flow::flush) encounters a problem, it returns one of
/// these variants to signal how the logger should react.
///
/// # Example
///
/// ```rust
/// # // This example is marked no_run due to OS execution policy.
/// use logger::HandlingKind;
///
/// let recoverable = HandlingKind::Ignore;
/// let fatal = HandlingKind::Fuse("disk full".into());
///
/// if let HandlingKind::Fuse(reason) = &fatal {
///     assert_eq!(reason, "disk full");
/// }
/// assert!(matches!(recoverable, HandlingKind::Ignore));
/// ```
#[derive(Debug)]
pub enum HandlingKind {
  /// An unrecoverable error that should cause the logger to panic.
  ///
  /// The contained string describes the reason for the failure.
  Fuse(String),
  /// A recoverable error that the logger should silently ignore.
  Ignore,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fuse_holds_reason() {
    let err = HandlingKind::Fuse("disk full".into());
    match err {
      HandlingKind::Fuse(reason) => assert_eq!(reason, "disk full"),
      _ => panic!("expected Fuse"),
    }
  }

  #[test]
  fn ignore_is_unit_like() {
    let err = HandlingKind::Ignore;
    match err {
      HandlingKind::Ignore => {}, // pass
      _ => panic!("expected Ignore"),
    }
  }
}
