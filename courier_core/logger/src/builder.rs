use std::{marker::PhantomData, sync::Arc};

use crate::{
  DefaultFormatter, Format, Logger,
  flow::{Flow, Identity, Stack},
};

#[doc(hidden)]
pub struct Empty;

#[doc(hidden)]
pub struct NonEmpty;

/// A builder for constructing a [`Logger`] with a fluent API.
///
/// The builder uses type-state generics to enforce correct usage at compile
/// time: you must call [`add_flow`](Builder::add_flow) and
/// [`capacity`](Builder::capacity) before [`build`](Builder::build) becomes
/// available.
///
/// # Type parameters
///
/// - `State` — tracks whether flows have been added ([`Empty`] / [`NonEmpty`]).
/// - `Formatter` — the formatter type (defaults to [`DefaultFormatter`]).
/// - `F` — the composite flow type (defaults to `()` before the first flow).
///
/// # Example
///
/// ```rust,no_run
/// use logger::{Builder, Level};
///
/// let logger = Builder::new()
///     .capacity(4096)
///     .add_flow(logger::flows::ConsoleFlow::new(Level::Debug))
///     .build_local()
///     .expect("Unable to build logger");
///
/// logger.info("Hello!".into(), "my_app");
/// drop(logger);
/// ```
pub struct Builder<State, Formatter = DefaultFormatter, F = ()> {
  capacity: Option<usize>,
  flows: Option<F>,
  format: Formatter,
  _state: PhantomData<State>,
}

impl Builder<Empty> {
  /// Creates a new [`Builder`] with the default settings.
  ///
  /// The returned builder has no flows, no capacity, and uses
  /// [`DefaultFormatter`] as its formatter.
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::Builder;
  ///
  /// let builder = Builder::new();
  /// ```
  pub fn new() -> Self {
    Builder {
      capacity: None,
      flows: None,
      format: DefaultFormatter,
      _state: PhantomData,
    }
  }
}

impl Default for Builder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

impl<Formatter: Format> Builder<Empty, Formatter> {
  /// Sets the capacity of the internal log channel.
  ///
  /// The capacity determines how many log records can be buffered before the
  /// sending side blocks (or drops, for `try_send`).
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }

  /// Sets a custom [`Format`] formatter, replacing the default one.
  ///
  /// # Example
  ///
  /// ```rust
  /// use std::fmt::Write as _;
  /// use logger::Builder;
  ///
  /// let builder = Builder::new()
  ///     .format(|record: &logger::Record, _timestamp: &str, out: &mut String| {
  ///         write!(out, "[CUSTOM] {}", String::from_utf8_lossy(&record.content))
  ///     });
  /// ```
  pub fn format<For: Format>(self, formatter: For) -> Builder<Empty, For> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format> Builder<NonEmpty, Formatter> {
  /// Sets the capacity of the internal log channel.
  ///
  /// See [`Builder::capacity`] on the `Empty` state for details.
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }

  /// Sets a custom [`Format`] formatter, replacing the current one.
  ///
  /// See [`Builder::format`] on the `Empty` state for details.
  pub fn format<For: Format>(self, formatter: For) -> Builder<NonEmpty, For> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format> Builder<Empty, Formatter> {
  /// Adds a [`Flow`] to the logger pipeline.
  ///
  /// The first call transitions the builder from [`Empty`] to [`NonEmpty`],
  /// unlocking [`build`](Builder::build). Subsequent calls stack additional
  /// flows on top.
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///     .capacity(1024)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Trace))
  ///     .build_local()
  ///     .unwrap();
  /// ```
  pub fn add_flow<T>(
    self, flow: T,
  ) -> Builder<NonEmpty, Formatter, Stack<T, Identity>>
  where
    T: Flow + 'static,
  {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { inner: flow, next: Identity }),
      format: self.format,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format, F> Builder<NonEmpty, Formatter, F>
where
  F: Flow + 'static,
{
  /// Adds an additional [`Flow`] to the logger pipeline.
  ///
  /// Flows are stacked — when a record is logged, it passes through each
  /// flow in the order they were added (last added processes first).
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///     .capacity(1024)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Trace))
  ///     .build_local()
  ///     .unwrap();
  /// ```
  pub fn add_flow<T>(self, flow: T) -> Builder<NonEmpty, Formatter, Stack<T, F>>
  where
    T: Flow + 'static,
  {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { inner: flow, next: self.flows.unwrap() }),
      format: self.format,
      _state: PhantomData,
    }
  }

  /// Builds the logger and installs it as the global singleton.
  ///
  /// Once installed, the logger can be accessed via
  /// [`Logger::get_instance`] and the logging macros (`info!`, `warn!`,
  /// etc.) become usable.
  ///
  /// # Errors
  ///
  /// Returns `Err("Capacity is unknown")` if [`capacity`](Builder::capacity)
  /// was not called.
  ///
  /// # Panics
  ///
  /// Panics if a logger has already been installed via a previous call to
  /// [`build`](Builder::build).
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// Builder::new()
  ///     .capacity(4096)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .build()
  ///     .expect("Failed to install logger");
  /// ```
  pub fn build(self) -> Result<Arc<Logger>, &'static str> {
    let logger = Logger::new(
      self.capacity.ok_or("Capacity is unknown")?,
      self.flows.unwrap(),
      self.format,
    );
    Ok(logger.install())
  }

  /// Builds the logger without installing it as the global singleton.
  ///
  /// This is useful for testing or when you need a standalone logger
  /// instance that does not interfere with a globally-installed logger.
  ///
  /// # Errors
  ///
  /// Returns `Err("Capacity is unknown")` if [`capacity`](Builder::capacity)
  /// was not called.
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///     .capacity(128)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .build_local()
  ///     .expect("Unable to build logger");
  ///
  /// logger.info("test message".into(), "test");
  /// drop(logger);
  /// ```
  pub fn build_local(self) -> Result<Arc<Logger>, &'static str> {
    Ok(Arc::new(Logger::new(
      self.capacity.ok_or("Capacity is unknown")?,
      self.flows.unwrap(),
      self.format,
    )))
  }
}

#[cfg(test)]
mod tests {
  use std::fmt::Write as _;
  use std::sync::{Arc, Mutex};

  use super::*;
  use crate::{HandlingKind, Level, Record, flow::Flow};

  struct TestFlow {
    level: Level,
    output: Arc<Mutex<Vec<String>>>,
    name: &'static str,
  }

  impl TestFlow {
    fn new(
      level: Level, name: &'static str, output: Arc<Mutex<Vec<String>>>,
    ) -> Self {
      Self { level, output, name }
    }
  }

  impl Flow for TestFlow {
    fn println(
      &mut self, _record: &Record, line: &str,
    ) -> Result<(), HandlingKind> {
      self.output.lock().unwrap().push(line.to_owned());
      Ok(())
    }

    fn flush(&mut self) -> Result<(), HandlingKind> {
      Ok(())
    }

    fn level(&self) -> Level {
      self.level
    }

    fn name(&self) -> &'static str {
      self.name
    }
  }

  #[test]
  fn builder_without_capacity_returns_error() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "test", output);
    let result = Builder::new().add_flow(flow).build_local();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Capacity is unknown");
  }

  #[test]
  fn builder_full_chain_succeeds() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "test_flow", output.clone());

    let logger =
      Builder::new().capacity(128).add_flow(flow).build_local().unwrap();

    logger.info("builder test".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("builder test"));
  }

  #[test]
  fn builder_multiple_flows_stacked() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow1 = TestFlow::new(Level::Trace, "flow1", output.clone());
    let flow2 = TestFlow::new(Level::Trace, "flow2", output.clone());

    let logger = Builder::new()
      .capacity(256)
      .add_flow(flow1)
      .add_flow(flow2)
      .build_local()
      .unwrap();

    logger.info("multi-flow test".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    // When both flows can_log (level Trace >= Info), they chain
    // flow2 processes first, then flow1 processes the result
    // So we should see 2 entries
    assert!(!logs.is_empty());
    assert!(logs.iter().any(|m| m.contains("multi-flow test")));
  }

  #[test]
  fn builder_custom_format() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "cf", output.clone());

    let logger = Builder::new()
      .capacity(128)
      .format(|record: &Record, _timestamp: &str, out: &mut String| {
        write!(out, "[CUSTOM] {}", String::from_utf8_lossy(&record.content))
      })
      .add_flow(flow)
      .build_local()
      .unwrap();

    logger.info("fmt test".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("[CUSTOM]"));
  }

  #[test]
  fn default_formatter_output_format() {
    use crate::Format;

    let record = Record::new("test msg".into(), Level::Info, "test");
    let formatter = DefaultFormatter;
    let mut line = String::new();
    formatter.format(&record, "2026-01-02 03:04:05", &mut line).unwrap();

    // Format: [YYYY-MM-DD HH:MM:SS][INFO][test] test msg
    assert_eq!(line, "[2026-01-02 03:04:05][INFO][test] test msg\n");
    assert!(line.starts_with('['));
    assert!(line.contains("][INFO][test] test msg"));
  }
}
