use std::sync::{Arc, Mutex};

use logger::{Builder, Flow, HandlingKind, Level, Logger, Record};

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

/// Dropping the logger is synchronous: the worker drains every queued record,
/// flushes the flows and is joined before `drop` returns.
fn flush_and_drop(logger: Arc<Logger>) {
  drop(logger);
}

// ---- Tests that use build_local() (no global interference) ----

#[test]
fn test_full_pipeline() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger = Builder::new().capacity(128).add_flow(flow).build_local();

  logger.log(Record::new(Level::Trace, "test", "trace msg"));
  logger.log(Record::new(Level::Debug, "test", "debug msg"));
  logger.log(Record::new(Level::Info, "test", "info msg"));
  logger.log(Record::new(Level::Warn, "test", "warn msg"));
  logger.log(Record::new(Level::Error, "test", "error msg"));
  logger.log(Record::new(Level::Fatal, "test", "fatal msg"));

  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 6, "all 6 levels should be logged");
  assert!(logs[0].contains("trace msg"));
  assert!(logs[1].contains("debug msg"));
  assert!(logs[2].contains("info msg"));
  assert!(logs[3].contains("warn msg"));
  assert!(logs[4].contains("error msg"));
  assert!(logs[5].contains("fatal msg"));
}

#[test]
fn test_multiple_loggers_independent() {
  let out1 = Arc::new(Mutex::new(Vec::new()));
  let out2 = Arc::new(Mutex::new(Vec::new()));

  let logger1 = Builder::new()
    .capacity(64)
    .add_flow(MockFlow::new(Level::Trace, out1.clone()))
    .build_local();
  let logger2 = Builder::new()
    .capacity(64)
    .add_flow(MockFlow::new(Level::Trace, out2.clone()))
    .build_local();

  logger1.log(Record::new(Level::Info, "test", "from logger1"));
  logger2.log(Record::new(Level::Info, "test", "from logger2"));

  flush_and_drop(logger1);
  flush_and_drop(logger2);

  assert_eq!(out1.lock().unwrap().len(), 1);
  assert_eq!(out2.lock().unwrap().len(), 1);
}

#[test]
fn test_level_filtering() {
  let output = Arc::new(Mutex::new(Vec::new()));
  // MockFlow at WARN level — only WARN, ERROR, FATAL messages pass
  let flow = MockFlow::new(Level::Warn, output.clone());

  let logger = Builder::new().capacity(128).add_flow(flow).build_local();

  logger.log(Record::new(Level::Info, "test", "this should be filtered"));
  logger.log(Record::new(Level::Debug, "test", "filtered too"));
  logger.log(Record::new(Level::Trace, "test", "also filtered"));
  logger.log(Record::new(Level::Warn, "test", "this should pass"));
  logger.log(Record::new(Level::Error, "test", "error passes too"));
  logger.log(Record::new(Level::Fatal, "test", "fatal also passes"));

  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 3, "only warn, error, fatal should pass");
  assert!(logs.iter().any(|m| m.contains("this should pass")));
  assert!(logs.iter().any(|m| m.contains("error passes too")));
  assert!(logs.iter().any(|m| m.contains("fatal also passes")));
}

#[test]
fn test_drop_cleanly_terminates() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger = Builder::new().capacity(64).add_flow(flow).build_local();

  logger.log(Record::new(Level::Info, "test", "before drop"));
  // Drop should not hang
  flush_and_drop(logger);

  assert_eq!(output.lock().unwrap().len(), 1);
}

#[test]
fn test_logger_with_target_routing() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger = Builder::new().capacity(128).add_flow(flow).build_local();

  logger.log(Record::with_target(Level::Info, "test", "mock", "targeted log"));
  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 1);
  assert!(logs[0].contains("targeted log"));
}

#[test]
fn test_custom_formatter() {
  use std::fmt::Write as _;

  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger = Builder::new()
    .capacity(128)
    .format(|record: &Record, _timestamp: &str, out: &mut String| {
      let _ = write!(
        out,
        "[{}][{}] {}",
        record.namespace, record.level, record.content
      );
    })
    .add_flow(flow)
    .build_local();

  logger.log(Record::new(Level::Info, "myns", "custom format msg"));
  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 1);
  assert!(logs[0].starts_with("[myns][INFO]"));
  assert!(logs[0].contains("custom format msg"));
}

// ---- Test that uses build() (global installation) ----
// NOTE: Only ONE test in this file can call build() because
// OnceLock can only be set once per process.

#[test]
fn test_global_install_and_macros() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  // Install globally — this is the only test that does this
  let logger = Builder::new().capacity(256).add_flow(flow).build();

  // Test instance() works
  assert!(Logger::instance().is_some());

  // Test all levels (positional namespace)
  logger::trace!(namespace = "test", "trace from macro");
  logger::debug!(namespace = "test", "debug from macro");
  logger::info!(namespace = "test", "info from macro");
  logger::warn!(namespace = "test", "warn from macro");
  logger::error!(namespace = "test", "error from macro");
  logger::fatal!(namespace = "test", "fatal from macro");

  // Test single-arg macros (default ns = env!("CARGO_PKG_NAME"))
  let formatted_val = "formatted";
  logger::info!("single arg static");
  logger::info!("single arg: {}", formatted_val);

  // Test positional template macros
  logger::info!(namespace = "test", "template: {}", "hello");
  logger::info!(namespace = "test", "named: {name}", name = "world");
  logger::info!(namespace = "test", "expr: {}", 1 + 2 * 3);

  // Test bare static via the namespace form
  logger::info!(namespace = "test", "bare static");
  logger::info!(namespace = "test", "named fmt: {}", 42);

  // Test target macros (default ns)
  logger::warn!(target = "mock", "targeted static");
  logger::warn!(target = "mock", "targeted fmt: {}", 42);
  // Test target macros (named ns)
  logger::info!(target = "mock", namespace = "test", "targeted: {}", 42);
  logger::error!(
    target = "mock",
    namespace = "test",
    "targeted fmt: {:.1}",
    2.8
  );
  logger::info!(target = "mock", namespace = "ns", "target + named ns");

  // Shut the global logger down so every queued record reached the flow.
  Logger::instance().unwrap().shutdown();

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 18);
  assert!(logs.iter().any(|m| m.contains("trace from macro")));
  assert!(logs.iter().any(|m| m.contains("debug from macro")));
  assert!(logs.iter().any(|m| m.contains("info from macro")));
  assert!(logs.iter().any(|m| m.contains("warn from macro")));
  assert!(logs.iter().any(|m| m.contains("error from macro")));
  assert!(logs.iter().any(|m| m.contains("fatal from macro")));
  // single-arg
  assert!(logs.iter().any(|m| m.contains("single arg static")));
  assert!(logs.iter().any(|m| m.contains("single arg: formatted")));
  // positional template
  assert!(logs.iter().any(|m| m.contains("template: hello")));
  assert!(logs.iter().any(|m| m.contains("named: world")));
  assert!(logs.iter().any(|m| m.contains("expr: 7")));
  // named namespace
  assert!(logs.iter().any(|m| m.contains("bare static")));
  assert!(logs.iter().any(|m| m.contains("named fmt: 42")));
  // target
  assert!(logs.iter().any(|m| m.contains("targeted: 42")));
  assert!(logs.iter().any(|m| m.contains("targeted static")));
  assert!(logs.iter().any(|m| m.contains("targeted fmt: 42")));
  assert!(logs.iter().any(|m| m.contains("targeted fmt: 2.8")));
  assert!(logs.iter().any(|m| m.contains("target + named ns")));

  // Clean up
  drop(logger);
}
