use std::{
  fmt::Write as _,
  sync::{Arc, Mutex, OnceLock},
  thread::JoinHandle,
};

use chrono::DateTime;
use crossbeam_channel::{Sender, bounded};

use crate::{Format, HandlingKind, Level, Record, flow::Flow};

static LOGGER: OnceLock<Arc<Logger>> = OnceLock::new();

/// The core logger that manages a background thread for asynchronous log
/// processing.
///
/// A `Logger` holds an internal MPSC channel and spawns a dedicated thread
/// that renders each incoming [`Record`] into its complete textual line
/// through the configured [`Format`] and dispatches that line to the
/// registered [`Flow`]s.
///
/// # Performance model
///
/// - **Call-site gating** — records below the logger's minimum accepted
///   level are discarded before the payload is formatted and before they
///   cross the channel.
/// - **Single-pass rendering** — the worker renders the *complete* line
///   through the configured [`Format`] (which has full control of the
///   output) into a reusable buffer, then flows write it verbatim;
///   steady-state formatting allocates nothing and no sink re-formats.
/// - **Cached timestamps** — the `YYYY-MM-DD HH:MM:SS` prefix is computed at
///   most once per wall-clock second on the worker thread.
/// - **Idle cost** — the worker blocks on the channel and wakes only when a
///   record arrives or the logger is dropped.
///
/// # Lifecycle
///
/// The logger thread runs until the `Logger` is dropped or
/// [`shutdown`](Self::shutdown) is called, at which point the internal
/// sender is dropped, the worker drains every queued record, flushes all
/// flows, exits, and is joined — so `drop` (or `shutdown`) returning
/// guarantees all previously sent records reached their sinks.
///
/// A `Logger` installed as the global singleton is never dropped, because
/// the singleton keeps an `Arc` alive for the whole process. Such a logger
/// must therefore be closed explicitly with
/// [`shutdown`](Self::shutdown) — otherwise the queued records and the
/// final flush are lost when the process exits.
///
/// # Global singleton
///
/// A `Logger` can be installed as a global singleton via
/// [`Builder::build`](crate::Builder::build). The convenience macros
/// (`info!`, `warn!`, etc.) call [`Logger::get_instance`] internally, so
/// they only work after installation.
///
/// # Example
///
/// ```rust
/// use logger::{Logger, Level};
///
/// let logger = Logger::new(
///     128,
///     logger::flows::ConsoleFlow::new(Level::Info),
///     |_record: &logger::Record, _timestamp: &str, _out: &mut String| Ok(()),  // no-op formatter
/// );
///
/// logger.info("Hello!".into(), "my_app");
/// drop(logger);
/// ```
#[derive(Debug)]
pub struct Logger {
  handle: Mutex<Option<JoinHandle<()>>>,
  /// The send half of the record channel. It is taken by
  /// [`shutdown`](Self::shutdown), which closes the channel and lets the
  /// worker drain the remaining records before exiting.
  sender: Mutex<Option<Sender<Record>>>,
  min_level: Level,
}

impl Logger {
  /// Creates a new `Logger` instance without installing it as the global
  /// singleton.
  ///
  /// The logger spawns a background thread that will run until the logger is dropped.
  ///
  /// # Parameters
  ///
  /// - `capacity` — size of the internal MPSC channel buffer.
  /// - `flows` — the composite flow (typically a [`Stack`](crate::flow::Stack)).
  /// - `format` — renders the complete textual line for each record.
  ///
  /// # Panics
  ///
  /// Panics if the internal thread cannot be spawned (e.g. the OS is out of
  /// resources).
  pub fn new<F, Formatter>(capacity: usize, flows: F, format: Formatter) -> Self
  where
    F: Flow,
    Formatter: Format,
  {
    let (sender, receiver) = bounded::<Record>(capacity);
    let min_level = flows.min_level();

    let handle = std::thread::Builder::new()
      .name("courier:logger".into())
      .stack_size(3 * 1024 * 1024)
      .spawn(move || {
        let mut flows = flows;
        let format = format;

        // Worker-local reusable buffers. The timestamp string is recomputed
        // at most once per wall-clock second (records carry second
        // resolution), and the line buffer is rewritten in place by the
        // formatter for every record — steady-state formatting allocates
        // nothing here.
        let mut last_second = i64::MIN;
        let mut ts_buf = String::new();
        let mut line = String::new();

        while let Ok(data) = receiver.recv() {
          if !flows.can_log(&data) {
            continue;
          }

          if data.timestamp != last_second {
            last_second = data.timestamp;
            ts_buf.clear();
            let _ = write!(
              ts_buf,
              "{}",
              DateTime::from_timestamp(data.timestamp, 0)
                .unwrap()
                .format("%Y-%m-%d %H:%M:%S")
            );
          }

          line.clear();
          if format.format(&data, &ts_buf, &mut line).is_err() {
            continue;
          }

          match flows.println(&data, &line) {
            Ok(()) => continue,
            Err(HandlingKind::Fuse(reason)) => {
              panic!("Logger encountered an unrecoverable error: {}", reason)
            },
            Err(HandlingKind::Ignore) => continue,
          }
        }

        let _ = flows.flush();
      })
      .expect("Unable to spawn a logger thread");

    Self {
      handle: Mutex::new(Some(handle)),
      sender: Mutex::new(Some(sender)),
      min_level,
    }
  }

  /// Installs this logger as the global singleton.
  ///
  /// # Panics
  ///
  /// Panics if a logger has already been installed.
  pub fn install(self) -> Arc<Self> {
    let this = Arc::new(self);
    LOGGER.set(this.clone()).expect("Logger has already been installed");
    this
  }

  /// Returns a clone of the globally-installed logger.
  ///
  /// # Panics
  ///
  /// Panics if no logger has been installed yet.
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level, Logger};
  ///
  /// Builder::new()
  ///     .capacity(128)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .build()
  ///     .unwrap();
  ///
  /// let logger = Logger::get_instance();
  /// logger.info("Hello!".into(), "my_app");
  /// ```
  pub fn get_instance() -> Arc<Self> {
    match LOGGER.get() {
      Some(s) => s.clone(),
      None => panic!("Logger not initialized"),
    }
  }

  /// Returns a clone of the globally-installed logger if instance exists.
  ///
  /// # Returns
  /// `Option<Arc<Logger>>` is returned when instance exists, and otherwise, `None` is returned.
  ///
  /// # Example
  ///
  /// ```rust
  /// use logger::{Builder, Level, Logger};
  ///
  /// Builder::new()
  ///     .capacity(128)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .build()
  ///     .unwrap();
  ///
  /// let logger = Logger::try_get_instance();
  /// logger.unwrap().info("Hello!".into(), "my_app");
  /// ```
  pub fn try_get_instance() -> Option<Arc<Self>> {
    LOGGER.get().cloned()
  }

  /// Returns `true` when the globally-installed logger could emit a record at
  /// `level`.
  ///
  /// This is the call-site gate used by the logging macros: when it returns
  /// `false` the payload is never formatted and nothing crosses the channel.
  ///
  /// When no logger has been installed yet this always returns `true`, which
  /// preserves the documented behaviour of the plain macros (they panic via
  /// [`get_instance`](Logger::get_instance)); the `try_*` macros then no-op
  /// as usual.
  #[doc(hidden)]
  pub fn is_enabled(level: Level) -> bool {
    match LOGGER.get() {
      Some(logger) => level >= logger.min_level,
      None => true,
    }
  }

  fn log(&self, record: Record) {
    // Cheap instance gate: records this logger can never emit never reach
    // the channel (macros already skipped building them entirely).
    if record.level < self.min_level {
      return;
    }
    if let Ok(sender) = self.sender.lock()
      && let Some(sender) = sender.as_ref()
    {
      let _ = sender.try_send(record);
    }
  }

  /// Shuts the logger down.
  ///
  /// No record is accepted afterwards: the ones still queued are
  /// written, every flow is flushed and the worker thread is joined, so
  /// this returns only once every previously sent record has reached its
  /// sink. Calling it more than once is safe, and `drop` calls it too.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Builder, Level, Logger};
  ///
  /// let logger = Builder::new()
  ///     .capacity(128)
  ///     .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///     .build_local()
  ///     .unwrap();
  ///
  /// logger.info("Hello!".into(), "my_app");
  /// logger.shutdown();
  /// ```
  pub fn shutdown(&self) {
    // Dropping the sender disconnects the channel: the worker drains
    // every queued record, flushes all flows and exits. Joining
    // afterwards makes this fully synchronous — when it returns every
    // record has reached its sinks and the thread is gone.
    if let Ok(mut sender) = self.sender.lock() {
      sender.take();
    }

    if let Ok(mut handle) = self.handle.lock()
      && let Some(handle) = handle.take()
    {
      let _ = handle.join();
    }
  }

  /// Logs a message at [`Level::Trace`].
  pub fn trace(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Trace, namespace));
  }

  /// Logs a message at [`Level::Trace`] with a target for flow routing.
  pub fn trace_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Trace, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Trace`] without allocating.
  pub fn trace_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Trace, namespace));
  }

  /// Logs a `&'static str` at [`Level::Trace`] with a target, without
  /// allocating.
  pub fn trace_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Trace,
      namespace,
      target,
    ));
  }

  /// Logs a message at [`Level::Debug`].
  pub fn debug(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Debug, namespace));
  }

  /// Logs a message at [`Level::Debug`] with a target for flow routing.
  pub fn debug_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Debug, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Debug`] without allocating.
  pub fn debug_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Debug, namespace));
  }

  /// Logs a `&'static str` at [`Level::Debug`] with a target, without
  /// allocating.
  pub fn debug_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Debug,
      namespace,
      target,
    ));
  }

  /// Logs a message at [`Level::Info`].
  pub fn info(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Info, namespace));
  }

  /// Logs a message at [`Level::Info`] with a target for flow routing.
  pub fn info_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Info, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Info`] without allocating.
  pub fn info_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Info, namespace));
  }

  /// Logs a `&'static str` at [`Level::Info`] with a target, without
  /// allocating.
  pub fn info_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Info,
      namespace,
      target,
    ));
  }

  /// Logs a message at [`Level::Warn`].
  pub fn warn(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Warn, namespace));
  }

  /// Logs a message at [`Level::Warn`] with a target for flow routing.
  pub fn warn_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Warn, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Warn`] without allocating.
  pub fn warn_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Warn, namespace));
  }

  /// Logs a `&'static str` at [`Level::Warn`] with a target, without
  /// allocating.
  pub fn warn_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Warn,
      namespace,
      target,
    ));
  }

  /// Logs a message at [`Level::Error`].
  pub fn error(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Error, namespace));
  }

  /// Logs a message at [`Level::Error`] with a target for flow routing.
  pub fn error_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Error, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Error`] without allocating.
  pub fn error_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Error, namespace));
  }

  /// Logs a `&'static str` at [`Level::Error`] with a target, without
  /// allocating.
  pub fn error_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Error,
      namespace,
      target,
    ));
  }

  /// Logs a message at [`Level::Fatal`].
  pub fn fatal(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Fatal, namespace));
  }

  /// Logs a message at [`Level::Fatal`] with a target for flow routing.
  pub fn fatal_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Fatal, namespace, target));
  }

  /// Logs a `&'static str` at [`Level::Fatal`] without allocating.
  pub fn fatal_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Fatal, namespace));
  }

  /// Logs a `&'static str` at [`Level::Fatal`] with a target, without
  /// allocating.
  pub fn fatal_from_static_with_target(
    &self, content: &'static str, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_from_static_with_target(
      content,
      Level::Fatal,
      namespace,
      target,
    ));
  }
}

impl Drop for Logger {
  fn drop(&mut self) {
    self.shutdown();
  }
}

impl Default for Logger {
  fn default() -> Self {
    Self::new(
      1000,
      crate::flows::ConsoleFlow::new(Level::Info),
      crate::DefaultFormatter,
    )
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use crate::{Builder, HandlingKind, Level, Logger, Record, flow::Flow};

  /// A mock flow that captures formatted output into a shared Vec<String>.
  struct MockFlow {
    level: Level,
    output: Arc<Mutex<Vec<String>>>,
  }

  impl MockFlow {
    fn new(level: Level, output: Arc<Mutex<Vec<String>>>) -> Self {
      Self { level, output }
    }
  }

  impl Flow for MockFlow {
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
      "mock"
    }
  }

  #[test]
  fn logger_logs_at_all_levels() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(
      128,
      flow,
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );

    logger.trace("trace msg".into(), "test");
    logger.debug("debug msg".into(), "test");
    logger.info("info msg".into(), "test");
    logger.warn("warn msg".into(), "test");
    logger.error("error msg".into(), "test");
    logger.fatal("fatal msg".into(), "test");

    // Give the logger thread time to process
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 6);
    assert!(logs.iter().any(|m| m.contains("trace msg")));
    assert!(logs.iter().any(|m| m.contains("debug msg")));
    assert!(logs.iter().any(|m| m.contains("info msg")));
    assert!(logs.iter().any(|m| m.contains("warn msg")));
    assert!(logs.iter().any(|m| m.contains("error msg")));
    assert!(logs.iter().any(|m| m.contains("fatal msg")));
  }

  #[test]
  fn logger_with_target() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(
      128,
      flow,
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );

    logger.info_with_target("targeted msg".into(), "test", "mock");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("targeted msg"));
  }

  #[test]
  fn logger_from_static() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(
      128,
      flow,
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );

    logger.info_from_static("static msg", "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("static msg"));
  }

  #[test]
  fn multiple_loggers_can_be_created_and_dropped() {
    let output1 = Arc::new(Mutex::new(Vec::new()));
    let output2 = Arc::new(Mutex::new(Vec::new()));

    let logger1 = Logger::new(
      128,
      MockFlow::new(Level::Trace, output1.clone()),
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );
    logger1.info("logger1 msg".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger1);

    let logger2 = Logger::new(
      128,
      MockFlow::new(Level::Trace, output2.clone()),
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );
    logger2.info("logger2 msg".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger2);

    let logs1 = output1.lock().unwrap();
    let logs2 = output2.lock().unwrap();
    assert_eq!(logs1.len(), 1);
    assert_eq!(logs2.len(), 1);
  }

  #[test]
  fn builder_build_local_does_not_install() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());

    let logger =
      Builder::new().capacity(64).add_flow(flow).build_local().unwrap();

    logger.info("local builder msg".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
  }

  #[test]
  fn drop_cleanly_terminates_thread() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());

    let logger = Logger::new(
      128,
      flow,
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );
    logger.info("before drop".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Drop should not hang — the thread should be signaled and joined
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
  }

  #[test]
  fn logger_format_variants() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(
      128,
      flow,
      |record: &Record, _timestamp: &str, out: &mut String| {
        // Simulates a formatter that emits the raw payload verbatim.
        out.push_str(&String::from_utf8_lossy(&record.content));
        out.push('\n');
        Ok(())
      },
    );

    // Positional args
    logger.info(format!("hello {}", "world"), "test");
    // Named args
    logger.info(format!("x={x}, y={y}", x = 1, y = 2), "test");
    // Complex expressions
    logger.info(format!("result={}", 1 + 2 * 3), "test");
    // Multiple positional
    logger.info(format!("a={}, b={}, c={}", 1, "two", 2.8), "test");

    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 4);
    assert!(logs[0].contains("hello world"));
    assert!(logs[1].contains("x=1, y=2"));
    assert!(logs[2].contains("result=7"));
    assert!(logs[3].contains("a=1, b=two, c=2.8"));
  }
}
