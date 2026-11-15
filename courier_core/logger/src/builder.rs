use std::{marker::PhantomData, sync::Arc};

use crate::{
  DefaultFormatter, Format, Logger,
  config::DEFAULT_CAPACITY,
  flow::{Flow, Identity, Stack},
};

#[doc(hidden)]
pub struct Empty;

#[doc(hidden)]
pub struct NonEmpty;

/// A builder for constructing a [`Logger`] with a fluent API.
///
/// The builder uses type-state generics to enforce correct usage at compile
/// time: you must call [`add_flow`](Builder::add_flow) before
/// [`build`](Builder::build) becomes available. The capacity defaults to
/// `1024` and only has to be set to override it.
///
/// Flows are composed in the type system ([`Stack`]), so adding or removing a
/// flow is a compile-time decision; there is no dynamic flow registry.
///
/// # Type parameters
///
/// - `State` — tracks whether flows have been added ([`Empty`] / [`NonEmpty`]).
/// - `Formatter` — the formatter type (defaults to [`DefaultFormatter`]).
/// - `F` — the composite flow type (defaults to `()` before the first flow).
///
/// # Examples
///
/// ```rust,no_run
/// use logger::{Builder, Level};
///
/// let logger = Builder::new()
///   .capacity(4096)
///   .add_flow(logger::flows::ConsoleFlow::new(Level::Debug))
///   .build_local();
///
/// drop(logger);
/// ```
pub struct Builder<State, Formatter = DefaultFormatter, F = ()> {
  capacity: usize,
  flows: Option<F>,
  format: Formatter,
  _state: PhantomData<State>,
}

impl Builder<Empty> {
  /// Creates a new [`Builder`] with the default settings.
  ///
  /// The returned builder has no flows, uses the default capacity and uses
  /// [`DefaultFormatter`] as its formatter.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::Builder;
  ///
  /// let builder = Builder::new();
  /// ```
  pub fn new() -> Self {
    Builder {
      capacity: DEFAULT_CAPACITY,
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
  /// sending side drops them. A capacity of zero is promoted to one, because a
  /// zero-capacity channel only accepts a record while the worker is parked in
  /// `recv`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::Builder;
  ///
  /// let builder = Builder::new().capacity(2048);
  /// ```
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = capacity;
    self
  }

  /// Sets a custom [`Format`] formatter, replacing the default one.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::fmt::Write as _;
  /// use logger::Builder;
  ///
  /// let builder = Builder::new().format(
  ///   |record: &logger::Record, _timestamp: &str, out: &mut String| {
  ///     let _ = write!(out, "[CUSTOM] {}", record.content);
  ///   },
  /// );
  /// ```
  pub fn format<For: Format>(self, formatter: For) -> Builder<Empty, For> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }

  /// Adds a [`Flow`] to the logger pipeline.
  ///
  /// The first call transitions the builder from [`Empty`] to [`NonEmpty`],
  /// unlocking [`build`](Builder::build). Subsequent calls stack additional
  /// flows on top.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///   .capacity(1024)
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Trace))
  ///   .build_local();
  ///
  /// drop(logger);
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
  /// Sets the capacity of the internal log channel.
  ///
  /// See [`Builder::capacity`] on the `Empty` state for details.
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = capacity;
    self
  }

  /// Sets a custom [`Format`] formatter.
  ///
  /// See [`Builder::format`] on the `Empty` state for details.
  pub fn format<For: Format>(
    self, formatter: For,
  ) -> Builder<NonEmpty, For, F> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }

  /// Adds an additional [`Flow`] to the logger pipeline.
  ///
  /// Flows are stacked — when a record is logged, it passes through each flow
  /// in the order they were added (last added processes first).
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  /// use logger::flows::ConsoleFlow;
  ///
  /// let logger = Builder::new()
  ///   .capacity(1024)
  ///   .add_flow(ConsoleFlow::new(Level::Info))
  ///   .add_flow(ConsoleFlow::new(Level::Trace))
  ///   .build_local();
  ///
  /// drop(logger);
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
  /// Once installed, the logger can be accessed via [`Logger::instance`] and
  /// the logging macros (`info!`, `warn!`, etc.) become active.
  ///
  /// # Panics
  ///
  /// Panics if a logger has already been installed via a previous call to
  /// [`build`](Builder::build).
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use logger::{Builder, Level};
  ///
  /// Builder::new()
  ///   .capacity(4096)
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build();
  /// ```
  pub fn build(self) -> Arc<Logger> {
    Logger::new(self.capacity, self.flows.unwrap(), self.format).install()
  }

  /// Builds the logger without installing it as the global singleton.
  ///
  /// This is useful for testing or when you need a standalone logger instance
  /// that does not interfere with a globally-installed logger.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///   .capacity(128)
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build_local();
  ///
  /// drop(logger);
  /// ```
  pub fn build_local(self) -> Arc<Logger> {
    Arc::new(Logger::new(self.capacity, self.flows.unwrap(), self.format))
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
    name: &'static str,
    output: Arc<Mutex<Vec<String>>>,
  }

  impl TestFlow {
    fn new(
      level: Level, name: &'static str, output: Arc<Mutex<Vec<String>>>,
    ) -> Self {
      Self { level, name, output }
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
  fn builder_without_capacity_keeps_every_record() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "test", output.clone());

    // No explicit capacity: the default must be large enough to buffer two
    // back-to-back records, which used to be dropped with capacity zero.
    let logger = Builder::new().add_flow(flow).build_local();

    logger.log(Record::new(Level::Info, "test", "first"));
    logger.log(Record::new(Level::Info, "test", "second"));

    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 2);
    assert!(logs[0].contains("first"));
    assert!(logs[1].contains("second"));
  }

  #[test]
  fn builder_full_chain_succeeds() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "test_flow", output.clone());

    let logger = Builder::new().capacity(128).add_flow(flow).build_local();

    logger.log(Record::new(Level::Info, "test", "builder test"));
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
      .build_local();

    logger.log(Record::new(Level::Info, "test", "multi-flow test"));
    drop(logger);

    let logs = output.lock().unwrap();
    // Both flows accept the record, so it is written twice.
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|line| line.contains("multi-flow test")));
  }

  #[test]
  fn builder_custom_format() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = TestFlow::new(Level::Trace, "cf", output.clone());

    let logger = Builder::new()
      .capacity(128)
      .format(|record: &Record, _timestamp: &str, out: &mut String| {
        let _ = write!(out, "[CUSTOM] {}", record.content);
      })
      .add_flow(flow)
      .build_local();

    logger.log(Record::new(Level::Info, "test", "fmt test"));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("[CUSTOM] fmt test"));
  }

  #[test]
  fn default_formatter_output_format() {
    use crate::Format;

    let record = Record::new(Level::Info, "test", "test msg");
    let formatter = DefaultFormatter;
    let mut line = String::new();
    formatter.format(&record, "2026-01-02 03:04:05", &mut line);

    // Format: [YYYY-MM-DD HH:MM:SS][INFO][test] test msg
    assert_eq!(line, "[2026-01-02 03:04:05][INFO][test] test msg\n");
    assert!(line.starts_with('['));
    assert!(line.contains("][INFO][test] test msg"));
  }
}
