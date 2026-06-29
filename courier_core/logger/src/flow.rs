use crate::{HandlingKind, Level, record::Record};

/// A destination for log records.
///
/// Implement this trait to create custom log outputs (file, console, network,
/// etc.). Flows can be composed with [`Stack`] so that a single record is
/// dispatched to multiple destinations.
///
/// # Required methods
///
/// - [`println`](Flow::println) — handles a single record after formatting.
/// - [`flush`](Flow::flush) — flushes any buffered data.
/// - [`level`](Flow::level) — returns the minimum [`Level`] this flow accepts.
/// - [`name`](Flow::name) — returns a unique name used for target routing.
///
/// # Target routing
///
/// A record with a `target` set will only be accepted by flows whose
/// [`name`](Flow::name) matches that target (as implemented by the default
/// [`can_log`](Flow::can_log) method). Records without a target are processed
/// by every flow whose level filter passes.
///
/// # Example
///
/// ```rust
/// use logger::flow::Flow;
/// use logger::{HandlingKind, Level, Record};
///
/// struct MyFlow { min_level: Level }
///
/// impl Flow for MyFlow {
///     fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
///         // Write `record.content` somewhere.
///         Ok(record)
///     }
///
///     fn flush(&mut self) -> Result<(), HandlingKind> {
///         Ok(())
///     }
///
///     fn level(&self) -> Level {
///         self.min_level
///     }
///
///     fn name(&self) -> &'static str {
///         "my_flow"
///     }
/// }
/// ```
pub trait Flow: Send + 'static {
  /// Handles a single log record after formatting.
  ///
  /// Returns the record on success so it can be passed to the next flow in
  /// the chain.
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind>;

  /// Flushes any buffered data to the underlying destination.
  fn flush(&mut self) -> Result<(), HandlingKind>;

  /// Returns `true` if this flow should process the given record.
  ///
  /// The default implementation checks that the record's level is at least
  /// [`level()`](Flow::level) and that the flow's name matches the record's
  /// target (if one is set).
  fn can_log(&self, record: &Record) -> bool {
    self.name() == record.target.unwrap_or(self.name())
      && self.level() <= record.level
  }

  /// Returns the minimum [`Level`] this flow accepts.
  fn level(&self) -> Level;

  /// Returns the unique name of this flow, used for target routing.
  fn name(&self) -> &'static str;
}

/// A no-op flow that serves as the terminator of a [`Stack`].
///
/// `Identity` never accepts any record ([`can_log`](Flow::can_log) always
/// returns `false`) and exists only to mark the end of the flow chain.
#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    Ok(record)
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    Ok(())
  }

  fn can_log(&self, _record: &Record) -> bool {
    false
  }

  fn level(&self) -> Level {
    Level::Fatal
  }

  fn name(&self) -> &'static str {
    "Identity"
  }
}

/// A linked-list node that chains two flows together.
///
/// When a record is logged through a `Stack`, the `next` flow processes it
/// first, then `inner` processes the result. This creates a pipeline where
/// flows are visited from the last added to the first added.
///
/// `Stack` is constructed by [`Builder::add_flow`](crate::Builder::add_flow)
/// and is typically not instantiated directly.
///
/// # Example
///
/// ```rust
/// use logger::flow::{Flow, Identity, Stack};
/// use logger::{HandlingKind, Level, Record};
///
/// // Two structs each implementing Flow (omitted for brevity — see `Flow`).
/// # struct A(Level, &'static str);
/// # impl Flow for A {
/// #  fn println(&mut self, r: Record) -> Result<Record, HandlingKind> { Ok(r) }
/// #  fn flush(&mut self) -> Result<(), HandlingKind> { Ok(()) }
/// #  fn level(&self) -> Level { self.0 }
/// #  fn name(&self) -> &'static str { self.1 }
/// # }
/// # struct B(Level, &'static str);
/// # impl Flow for B {
/// #  fn println(&mut self, r: Record) -> Result<Record, HandlingKind> { Ok(r) }
/// #  fn flush(&mut self) -> Result<(), HandlingKind> { Ok(()) }
/// #  fn level(&self) -> Level { self.0 }
/// #  fn name(&self) -> &'static str { self.1 }
/// # }
/// let stack: Stack<B, Stack<A, Identity>> =
///     Stack { inner: B(Level::Trace, "b"), next: Stack { inner: A(Level::Info, "a"), next: Identity } };
/// assert_eq!(stack.name(), "b");
/// ```
pub struct Stack<F: Flow, S: Flow> {
  /// The flow added most recently (processes first in the chain).
  pub inner: F,
  /// The previously-added flow (or [`Identity`] for the first flow).
  pub next: S,
}

impl<F: Flow, S: Flow> Flow for Stack<F, S> {
  fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
    if self.inner.can_log(&record) && self.next.can_log(&record) {
      self.inner.println(self.next.println(record)?)
    } else if self.inner.can_log(&record) && !(self.next.can_log(&record)) {
      self.inner.println(record)
    } else if !(self.inner.can_log(&record)) && self.next.can_log(&record) {
      self.next.println(record)
    } else {
      Ok(record)
    }
  }

  fn flush(&mut self) -> Result<(), HandlingKind> {
    self.next.flush()?;
    self.inner.flush()
  }

  fn can_log(&self, record: &Record) -> bool {
    self.inner.can_log(record) | self.next.can_log(record)
  }

  fn level(&self) -> Level {
    self.inner.level()
  }

  fn name(&self) -> &'static str {
    self.inner.name()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Level;
  use bytes::Bytes;

  // ---- Test helpers ----

  /// A flow that always accepts logs above its level.
  struct TestFlow {
    level: Level,
    name: &'static str,
    prefix: &'static str,
    output: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
  }

  impl TestFlow {
    fn new(
      level: Level, name: &'static str, prefix: &'static str,
      output: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    ) -> Self {
      Self { level, name, prefix, output }
    }
  }

  impl Flow for TestFlow {
    fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
      let content = String::from_utf8(record.content.to_vec()).unwrap();
      self.output.lock().unwrap().push(format!("[{}]{}", self.prefix, content));
      Ok(record)
    }

    fn flush(&mut self) -> Result<(), HandlingKind> {
      self.output.lock().unwrap().push(format!("[{}]flushed", self.prefix));
      Ok(())
    }

    fn level(&self) -> Level {
      self.level
    }

    fn name(&self) -> &'static str {
      self.name
    }
  }

  fn make_record(msg: &str, target: Option<&'static str>) -> Record {
    Record {
      timestamp: 0,
      level: Level::Info,
      target,
      namespace: "test",
      content: Bytes::from(msg.to_string()),
    }
  }

  // ---- Identity tests ----

  #[test]
  fn identity_can_log_always_false() {
    let identity = Identity;
    let record = make_record("test", Some("any_target"));
    assert!(!identity.can_log(&record));
  }

  #[test]
  fn identity_println_passes_through() {
    let mut identity = Identity;
    let record = make_record("test", None);
    let result = identity.println(record).unwrap();
    assert_eq!(&result.content[..], b"test");
  }

  #[test]
  fn identity_level_is_fatal() {
    assert_eq!(Identity.level(), Level::Fatal);
  }

  #[test]
  fn identity_name_is_identity() {
    assert_eq!(Identity.name(), "Identity");
  }

  #[test]
  fn identity_flush_returns_ok() {
    let mut identity = Identity;
    assert!(identity.flush().is_ok());
  }

  // ---- Stack tests ----

  #[test]
  fn stack_both_can_log_chains_println() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("chain", None);
    stack.println(record).unwrap();

    let logs = output.lock().unwrap();
    // next processes first (inner.println(next.println(record)?)?)
    // Both flows see the same original record content since println passes it through
    assert_eq!(logs.len(), 2);
    assert!(logs[0].contains("[NEXT]chain"));
    assert!(logs[1].contains("[INNER]chain"));
  }

  #[test]
  fn stack_only_inner_can_log() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    // Use a level higher than Info so that the "next" flow rejects Info-level records
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Error, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("only_inner", None);
    stack.println(record).unwrap();

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("[INNER]only_inner"));
  }

  #[test]
  fn stack_only_next_can_log() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("only_next", None);
    stack.println(record).unwrap();

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("[NEXT]only_next"));
  }

  #[test]
  fn stack_neither_can_log_returns_record_unchanged() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Error, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("neither", None);
    let result = stack.println(record).unwrap();

    let logs = output.lock().unwrap();
    assert!(logs.is_empty());
    assert_eq!(&result.content[..], b"neither");
  }

  #[test]
  fn stack_can_log_is_or() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    // Info: inner (Error) can't, next (Trace) can → OR = true
    assert!(stack.can_log(&make_record("", None)));

    // Info with target matching inner's name though
    // can_log default: name() == record.target.unwrap_or(name()) && level() <= record.level
    // Since both use default can_log, need to check the logic
    // Actually, TestFlow uses the default Flow::can_log, which checks name match + level
    // For a record with no target (None), can_log checks name() == name() (always true)
    // So only level matters for None target
  }

  #[test]
  fn stack_can_log_with_target_filters_by_name() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    // Record targeted at "inner" — only inner can log
    let record = make_record("targeted", Some("inner"));
    // We can't easily test println details here, but we can check can_log
    assert!(stack.can_log(&record)); // inner can log, so OR is true

    // Record targeted at "both" — neither matches
    let record = make_record("targeted", Some("both"));
    assert!(!stack.can_log(&record));
  }

  #[test]
  fn stack_flush_flushes_next_then_inner() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    stack.flush().unwrap();

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0], "[NEXT]flushed"); // next flushed first
    assert_eq!(logs[1], "[INNER]flushed"); // then inner
  }

  #[test]
  fn stack_level_delegates_to_inner() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    assert_eq!(stack.level(), Level::Error);
  }

  #[test]
  fn stack_name_delegates_to_inner() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    assert_eq!(stack.name(), "inner");
  }
}
