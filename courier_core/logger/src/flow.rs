use crate::{HandlingKind, Level, record::Record};

/// A destination for log records.
///
/// Implement this trait to create custom log outputs (file, console, network,
/// etc.). Flows can be composed with [`Stack`] so that a single record is
/// dispatched to multiple destinations.
///
/// # Required methods
///
/// - [`println`](Flow::println) — handles a single formatted record.
/// - [`flush`](Flow::flush) — flushes any buffered data.
/// - [`level`](Flow::level) — returns the minimum [`Level`] this flow accepts.
/// - [`name`](Flow::name) — returns a unique name used for target routing.
///
/// # Single-pass rendering
///
/// The logger's worker thread renders each record **once** through the
/// configured [`Format`](crate::Format), producing the complete final text
/// (`line`) into a reusable buffer, and then hands every accepting flow the
/// same line. Flows are pure sinks: they write `line` verbatim and never
/// re-format the record.
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
///     fn println(&mut self, _record: &Record, line: &str) -> Result<(), HandlingKind> {
///         // `line` is the fully rendered output; write it wherever this flow sinks.
///         Ok(())
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
  /// Writes one fully rendered record.
  ///
  /// `line` is the complete output produced by the configured
  /// [`Format`](crate::Format) — timestamps, levels, namespaces and payload
  /// included, in whatever layout the formatter chose. Sinks must treat it as
  /// the final text and write it verbatim. The `record` argument is provided
  /// for metadata reads only; do not re-format from it.
  fn println(
    &mut self, record: &Record, line: &str,
  ) -> Result<(), HandlingKind>;

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

  /// Returns the lowest level at which this flow could *possibly* accept a
  /// record (ignoring target routing).
  ///
  /// Used to compute a cheap, conservative gate at the logging call site so
  /// that messages a logger can never emit are discarded before formatting
  /// and before crossing the channel. For a single flow this is exactly
  /// [`level()`](Flow::level); [`Stack`] overrides it with the minimum of
  /// both halves and [`Identity`] with `Fatal` (it never accepts anything).
  fn min_level(&self) -> Level {
    self.level()
  }
}

/// A no-op flow that serves as the terminator of a [`Stack`].
///
/// `Identity` never accepts any record ([`can_log`](Flow::can_log) always
/// returns `false`) and exists only to mark the end of the flow chain.
#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn println(
    &mut self, _record: &Record, _line: &str,
  ) -> Result<(), HandlingKind> {
    Ok(())
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
/// #  fn println(&mut self, _r: &Record, _h: &str) -> Result<(), HandlingKind> { Ok(()) }
/// #  fn flush(&mut self) -> Result<(), HandlingKind> { Ok(()) }
/// #  fn level(&self) -> Level { self.0 }
/// #  fn name(&self) -> &'static str { self.1 }
/// # }
/// # struct B(Level, &'static str);
/// # impl Flow for B {
/// #  fn println(&mut self, _r: &Record, _h: &str) -> Result<(), HandlingKind> { Ok(()) }
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
  fn println(
    &mut self, record: &Record, line: &str,
  ) -> Result<(), HandlingKind> {
    // Evaluate each node's filter exactly once, then dispatch to the
    // accepting subset. `next` processes first, then `inner`, mirroring the
    // previous value-passing pipeline (where `next` ran before `inner`).
    let inner_accepts = self.inner.can_log(record);
    let next_accepts = self.next.can_log(record);
    match (inner_accepts, next_accepts) {
      (true, true) => {
        self.next.println(record, line)?;
        self.inner.println(record, line)?;
      },
      (true, false) => self.inner.println(record, line)?,
      (false, true) => self.next.println(record, line)?,
      (false, false) => {},
    }
    Ok(())
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

  fn min_level(&self) -> Level {
    std::cmp::min(self.inner.min_level(), self.next.min_level())
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
    fn println(
      &mut self, _record: &Record, line: &str,
    ) -> Result<(), HandlingKind> {
      self.output.lock().unwrap().push(format!("[{}]{line}", self.prefix));
      Ok(())
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
  fn identity_println_is_ok() {
    let mut identity = Identity;
    let record = make_record("test", None);
    let result = identity.println(&record, "");
    assert!(result.is_ok());
    assert_eq!(&record.content[..], b"test");
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
  fn identity_min_level_is_fatal() {
    assert_eq!(Identity.min_level(), Level::Fatal);
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
    stack.println(&record, "chain\n").unwrap();

    let logs = output.lock().unwrap();
    // `next` processes first, then `inner`.
    assert_eq!(logs.len(), 2);
    assert!(logs[0].contains("[NEXT]chain"));
    assert!(logs[1].contains("[INNER]chain"));
  }

  #[test]
  fn stack_rendered_line_is_forwarded_to_all() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("payload", None);
    stack
      .println(&record, "[2026-01-02 03:04:05][INFO][test] payload\n")
      .unwrap();

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 2);
    assert!(logs[0].contains("[2026-01-02 03:04:05][INFO][test] payload"));
    assert!(logs[1].contains("[2026-01-02 03:04:05][INFO][test] payload"));
  }

  #[test]
  fn stack_only_inner_can_log() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    // Use a level higher than Info so that the "next" flow rejects Info-level records
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Error, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("only_inner", None);
    stack.println(&record, "only_inner\n").unwrap();

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
    stack.println(&record, "only_next\n").unwrap();

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("[NEXT]only_next"));
  }

  #[test]
  fn stack_neither_can_log_is_noop() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Error, "next", "NEXT", output.clone());
    let mut stack = Stack { inner, next };

    let record = make_record("neither", None);
    let result = stack.println(&record, "neither\n");
    assert!(result.is_ok());

    let logs = output.lock().unwrap();
    assert!(logs.is_empty());
    assert_eq!(&record.content[..], b"neither");
  }

  #[test]
  fn stack_can_log_is_or() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Error, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    // Info: inner (Error) can't, next (Trace) can → OR = true
    assert!(stack.can_log(&make_record("", None)));
  }

  #[test]
  fn stack_can_log_with_target_filters_by_name() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Trace, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    // Record targeted at "inner" — inner can log, so OR is true
    let record = make_record("targeted", Some("inner"));
    assert!(stack.can_log(&record));

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

  #[test]
  fn stack_min_level_is_min_of_children() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = TestFlow::new(Level::Info, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Trace, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };

    assert_eq!(stack.min_level(), Level::Trace);

    let inner = TestFlow::new(Level::Warn, "inner", "INNER", output.clone());
    let next = TestFlow::new(Level::Error, "next", "NEXT", output.clone());
    let stack = Stack { inner, next };
    assert_eq!(stack.min_level(), Level::Warn);
  }
}
