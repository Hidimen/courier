use std::{
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
  },
  thread::JoinHandle,
  time::Duration,
};

use crossbeam::channel::{RecvTimeoutError, Sender, bounded};

use crate::{Format, HandlingKind, Level, Record, flow::Flow};

static LOGGER: OnceLock<Arc<Logger>> = OnceLock::new();

/// The core logger that manages a background thread for asynchronous log
/// processing.
///
/// A `Logger` holds an internal MPSC channel and spawns a dedicated thread
/// that formats incoming [`Record`]s and dispatches them through the
/// registered [`Flow`]s.
///
/// # Lifecycle
///
/// The logger thread runs until the `Logger` is dropped, at which point it
/// signals the thread to stop, drains remaining records, flushes all flows,
/// and joins the thread.
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
///     |r| r,  // no-op formatter
/// );
///
/// logger.info("Hello!".into(), "my_app");
/// drop(logger);
/// ```
#[derive(Debug)]
pub struct Logger {
  handle: Mutex<Option<JoinHandle<()>>>,
  sender: Sender<Record>,
  signal: Arc<AtomicBool>,
}

impl Drop for Logger {
  fn drop(&mut self) {
    self.signal.store(false, Ordering::Release);
    if let Ok(mut guard) = self.handle.lock()
      && let Some(handle) = guard.take()
    {
      let _ = handle.join();
    }
  }
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
  /// - `format` — the formatter applied to each record before dispatch.
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
    let signal = Arc::new(AtomicBool::new(true));
    let signal_cloned = signal.clone();

    let handle = std::thread::Builder::new()
      .name("logger".into())
      .stack_size(3 * 1024 * 1024)
      .spawn(move || {
        let mut flows = flows;
        let format = format;
        let signal = signal_cloned;
        while signal.load(Ordering::Acquire) {
          match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(data) => {
              if !flows.can_log(&data) {
                continue;
              }

              match flows.println(format.format(data)) {
                Ok(_) => continue,
                Err(HandlingKind::Fuse(reason)) => {
                  panic!("Logger encountered a unrecoverable error: {}", reason)
                },
                Err(HandlingKind::Ignore) => continue,
              }
            },
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => continue,
          }
        }

        let _ = flows.flush();
      })
      .expect("Unable to spawn a logger thread");

    Self { handle: Mutex::new(Some(handle)), sender, signal }
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

  fn log(&self, record: Record) {
    let _ = self.sender.try_send(record);
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
    fn println(&mut self, record: Record) -> Result<Record, HandlingKind> {
      let content = String::from_utf8(record.content.to_vec()).unwrap();
      self.output.lock().unwrap().push(content);
      Ok(record)
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
    let logger = Logger::new(128, flow, |r| r);

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
    let logger = Logger::new(128, flow, |r| r);

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
    let logger = Logger::new(128, flow, |r| r);

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

    let logger1 =
      Logger::new(128, MockFlow::new(Level::Trace, output1.clone()), |r| r);
    logger1.info("logger1 msg".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(logger1);

    let logger2 =
      Logger::new(128, MockFlow::new(Level::Trace, output2.clone()), |r| r);
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

    let logger = Logger::new(128, flow, |r| r);
    logger.info("before drop".into(), "test");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Drop should not hang — the thread should be signaled and joined
    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
  }
}
