use std::{
  fmt::Write as _,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
  },
  thread::JoinHandle,
  time::{Duration, Instant},
};

use chrono::Utc;
use crossbeam_channel::{SendTimeoutError, Sender, bounded};

use crate::{Format, HandlingKind, Level, Record, flow::Flow};

static LOGGER: OnceLock<Arc<Logger>> = OnceLock::new();

/// A message crossing the logger channel.
///
/// Records and the shutdown notification share one channel so the worker
/// always observes them in order: every record queued before `Shutdown` is
/// drained before the flows are flushed.
enum Message {
  /// A record to render and dispatch.
  Record(Record),
  /// Stops the worker after the records queued before it have been written.
  Shutdown,
}

/// The core logger that manages a background thread for asynchronous log
/// processing.
///
/// A `Logger` owns a bounded MPSC channel and a dedicated thread that renders
/// each incoming [`Record`] into its complete textual line through the
/// configured [`Format`] and dispatches that line to the registered
/// [`Flow`]s.
///
/// # Performance model
///
/// - **Call-site gating** — records below the logger's minimum accepted level
///   are discarded before the payload is formatted and before they cross the
///   channel.
/// - **Lock-free send** — the sender is stored directly, so logging a record
///   costs one channel `try_send` instead of a mutex acquisition.
/// - **Deferred timestamps** — records do not carry a timestamp; the worker
///   stamps them when they are dequeued, so the call site never reads a clock.
/// - **Single-pass rendering** — the worker renders the *complete* line
///   through the configured [`Format`] into a reusable buffer, then flows
///   write it verbatim; steady-state formatting allocates nothing.
/// - **Cached timestamps** — the `YYYY-MM-DD HH:MM:SS` prefix is computed at
///   most once per wall-clock second on the worker thread.
///
/// # Lifecycle
///
/// The logger thread runs until [`shutdown`](Self::shutdown) is called or the
/// `Logger` is dropped. Both paths are synchronous: the worker drains every
/// queued record, flushes all flows, exits, and is joined — so returning from
/// `shutdown` guarantees all previously accepted records reached their sinks.
///
/// A `Logger` installed as the global singleton is never dropped, because the
/// singleton keeps an `Arc` alive for the whole process. Such a logger must
/// therefore be closed explicitly with [`shutdown`](Self::shutdown).
///
/// # Global singleton
///
/// A `Logger` can be installed as a global singleton via
/// [`Builder::build`](crate::Builder::build). The convenience macros
/// (`info!`, `warn!`, etc.) look it up through [`Logger::instance`], and
/// become no-ops when no logger has been installed.
///
/// # Examples
///
/// ```rust,no_run
/// use logger::{Builder, Level, Record};
///
/// let logger = Builder::new()
///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
///   .build_local();
///
/// logger.log(Record::new(Level::Info, "my_app", "Hello!"));
/// drop(logger);
/// ```
pub struct Logger {
  sender: Sender<Message>,
  closed: AtomicBool,
  handle: Mutex<Option<JoinHandle<()>>>,
  min_level: Level,
}

impl Logger {
  /// Creates a new `Logger` instance without installing it as the global
  /// singleton.
  ///
  /// The logger spawns a background thread that runs until the logger is
  /// dropped or [`shutdown`](Self::shutdown) is called.
  ///
  /// # Parameters
  ///
  /// - `capacity` — size of the internal channel buffer. Zero is promoted to
  ///   one, because a zero-capacity channel only accepts a record while the
  ///   worker is parked in `recv`.
  /// - `flows` — the composite flow (typically a [`Stack`](crate::flow::Stack)).
  /// - `format` — renders the complete textual line for each record.
  ///
  /// # Panics
  ///
  /// Panics if the internal thread cannot be spawned (e.g. the OS is out of
  /// resources).
  pub(crate) fn new<F, Formatter>(
    capacity: usize, flows: F, format: Formatter,
  ) -> Self
  where
    F: Flow,
    Formatter: Format,
  {
    let (sender, receiver) = bounded::<Message>(capacity.max(1));
    let min_level = flows.min_level();

    let handle = std::thread::Builder::new()
      .name("courier:logger".into())
      .stack_size(3 * 1024 * 1024)
      .spawn(move || {
        let mut flows = flows;
        let format = format;

        // Worker-local reusable buffers. The timestamp string is recomputed
        // at most once per wall-clock second, and the line buffer is rewritten
        // in place by the formatter for every record — steady-state formatting
        // allocates nothing here.
        let mut last_second = i64::MIN;
        let mut ts_buf = String::new();
        let mut line = String::new();

        while let Ok(Message::Record(record)) = receiver.recv() {
          if !flows.can_log(&record) {
            continue;
          }

          let now = Utc::now();
          if now.timestamp() != last_second {
            last_second = now.timestamp();
            ts_buf.clear();
            let _ = write!(ts_buf, "{}", now.format("%Y-%m-%d %H:%M:%S"));
          }

          line.clear();
          format.format(&record, &ts_buf, &mut line);

          match flows.println(&record, &line) {
            Ok(()) | Err(HandlingKind::Ignore) => {},
            Err(HandlingKind::Fuse(reason)) => {
              panic!("Logger encountered an unrecoverable error: {}", reason)
            },
          }
        }

        let _ = flows.flush();
      })
      .expect("Unable to spawn a logger thread");

    Self {
      sender,
      closed: AtomicBool::new(false),
      handle: Mutex::new(Some(handle)),
      min_level,
    }
  }

  /// Installs this logger as the global singleton.
  ///
  /// # Panics
  ///
  /// Panics if a logger has already been installed.
  pub(crate) fn install(self) -> Arc<Self> {
    let this = Arc::new(self);
    if LOGGER.set(this.clone()).is_err() {
      panic!("Logger has already been installed");
    }
    this
  }

  /// Returns the globally installed logger, if any.
  ///
  /// This is the call-site gate used by the logging macros: when it returns
  /// `None` the macros do nothing, so logging before installation is safe.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use logger::{Builder, Level, Logger};
  ///
  /// assert!(Logger::instance().is_none());
  ///
  /// Builder::new()
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build();
  ///
  /// assert!(Logger::instance().is_some());
  /// ```
  pub fn instance() -> Option<&'static Self> {
    LOGGER.get().map(|logger| logger.as_ref())
  }

  /// Returns `true` when this logger could emit a record at `level`.
  ///
  /// This is the call-site gate used by the logging macros: when it returns
  /// `false` the payload is never formatted and nothing crosses the channel.
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use logger::{Builder, Level};
  ///
  /// let logger = Builder::new()
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build_local();
  ///
  /// assert!(logger.enabled(Level::Error));
  /// assert!(!logger.enabled(Level::Debug));
  /// ```
  pub fn enabled(&self, level: Level) -> bool {
    level >= self.min_level
  }

  /// Queues a record for the worker thread.
  ///
  /// The record is dropped when the logger has been shut down, when its level
  /// is below the logger's minimum accepted level, or when the channel is
  /// full. Sending never blocks, so this is safe to call from async code.
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use logger::{Builder, Level, Record};
  ///
  /// let logger = Builder::new()
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build_local();
  ///
  /// logger.log(Record::new(Level::Info, "my_app", "hello"));
  /// drop(logger);
  /// ```
  pub fn log(&self, record: Record) {
    // Cheap gate: records this logger can never emit never reach the channel
    // (the macros already skipped building them entirely).
    if self.closed.load(Ordering::Relaxed) {
      return;
    }
    if record.level < self.min_level {
      return;
    }
    let _ = self.sender.try_send(Message::Record(record));
  }

  /// Shuts the logger down.
  ///
  /// No record is accepted afterwards: the ones still queued are written,
  /// every flow is flushed and the worker thread is joined, so this returns
  /// only once every previously accepted record has reached its sink.
  /// Calling it more than once is safe, and `drop` calls it too.
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use logger::{Builder, Level, Record};
  ///
  /// let logger = Builder::new()
  ///   .add_flow(logger::flows::ConsoleFlow::new(Level::Info))
  ///   .build_local();
  ///
  /// logger.log(Record::new(Level::Info, "my_app", "bye"));
  /// logger.shutdown();
  /// ```
  pub fn shutdown(&self) {
    self.closed.store(true, Ordering::SeqCst);

    // Wake the worker so it drains the queue and exits. If the channel is
    // full, retry for a bounded time instead of blocking the caller forever.
    if self.sender.try_send(Message::Shutdown).is_err() {
      let deadline = Instant::now() + Duration::from_millis(500);
      while Instant::now() < deadline {
        match self
          .sender
          .send_timeout(Message::Shutdown, Duration::from_millis(10))
        {
          Ok(()) | Err(SendTimeoutError::Disconnected(_)) => break,
          Err(SendTimeoutError::Timeout(_)) => {},
        }
      }
    }

    if let Ok(mut handle) = self.handle.lock()
      && let Some(handle) = handle.take()
    {
      let _ = handle.join();
    }
  }
}

impl Drop for Logger {
  fn drop(&mut self) {
    self.shutdown();
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use crate::{HandlingKind, Level, Logger, Record, flow::Flow};

  /// A mock flow that captures formatted output into a shared `Vec<String>`.
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

  /// A formatter that writes the raw payload verbatim.
  fn payload_only(record: &Record, _timestamp: &str, out: &mut String) {
    out.push_str(&record.content);
    out.push('\n');
  }

  #[test]
  fn logger_logs_at_all_levels() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(128, flow, payload_only);

    logger.log(Record::new(Level::Trace, "test", "trace msg"));
    logger.log(Record::new(Level::Debug, "test", "debug msg"));
    logger.log(Record::new(Level::Info, "test", "info msg"));
    logger.log(Record::new(Level::Warn, "test", "warn msg"));
    logger.log(Record::new(Level::Error, "test", "error msg"));
    logger.log(Record::new(Level::Fatal, "test", "fatal msg"));

    // Dropping drains the queue synchronously, so no sleep is needed.
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
    let logger = Logger::new(128, flow, payload_only);

    logger.log(Record::with_target(
      Level::Info,
      "test",
      "mock",
      "targeted msg",
    ));
    logger.log(Record::with_target(
      Level::Info,
      "test",
      "other",
      "ignored msg",
    ));

    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("targeted msg"));
  }

  #[test]
  fn zero_capacity_still_delivers_a_record() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    // A raw zero-capacity channel would drop the record unless the worker was
    // already parked in `recv`; the logger promotes zero to one.
    let logger = Logger::new(0, flow, payload_only);

    logger.log(Record::new(Level::Info, "test", "first"));

    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("first"));
  }

  #[test]
  fn multiple_loggers_can_be_created_and_dropped() {
    let output1 = Arc::new(Mutex::new(Vec::new()));
    let output2 = Arc::new(Mutex::new(Vec::new()));

    let logger1 = Logger::new(
      128,
      MockFlow::new(Level::Trace, output1.clone()),
      payload_only,
    );
    logger1.log(Record::new(Level::Info, "test", "logger1 msg"));
    drop(logger1);

    let logger2 = Logger::new(
      128,
      MockFlow::new(Level::Trace, output2.clone()),
      payload_only,
    );
    logger2.log(Record::new(Level::Info, "test", "logger2 msg"));
    drop(logger2);

    assert_eq!(output1.lock().unwrap().len(), 1);
    assert_eq!(output2.lock().unwrap().len(), 1);
  }

  #[test]
  fn shutdown_is_idempotent() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(128, flow, payload_only);

    logger.log(Record::new(Level::Info, "test", "before shutdown"));
    logger.shutdown();
    logger.shutdown();

    assert_eq!(output.lock().unwrap().len(), 1);
  }

  #[test]
  fn logger_format_variants() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let flow = MockFlow::new(Level::Trace, output.clone());
    let logger = Logger::new(128, flow, payload_only);

    // Positional args
    logger.log(Record::new(Level::Info, "test", format!("hello {}", "world")));
    // Named args
    logger.log(Record::new(
      Level::Info,
      "test",
      format!("x={x}, y={y}", x = 1, y = 2),
    ));
    // Complex expressions
    logger.log(Record::new(
      Level::Info,
      "test",
      format!("result={}", 1 + 2 * 3),
    ));
    // Multiple positional
    logger.log(Record::new(
      Level::Info,
      "test",
      format!("a={}, b={}, c={}", 1, "two", 2.8),
    ));

    drop(logger);

    let logs = output.lock().unwrap();
    assert_eq!(logs.len(), 4);
    assert!(logs[0].contains("hello world"));
    assert!(logs[1].contains("x=1, y=2"));
    assert!(logs[2].contains("result=7"));
    assert!(logs[3].contains("a=1, b=two, c=2.8"));
  }
}
