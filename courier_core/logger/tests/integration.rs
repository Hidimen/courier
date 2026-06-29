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

/// Helper: wait a bit for the logger thread to process messages, then drop.
fn flush_and_drop(logger: Arc<Logger>) {
  std::thread::sleep(std::time::Duration::from_millis(150));
  drop(logger);
}

// ---- Tests that use build_local() (no global interference) ----

#[test]
fn test_full_pipeline() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger =
    Builder::new().capacity(128).add_flow(flow).build_local().unwrap();

  logger.trace("trace msg".into(), "test");
  logger.debug("debug msg".into(), "test");
  logger.info("info msg".into(), "test");
  logger.warn("warn msg".into(), "test");
  logger.error("error msg".into(), "test");
  logger.fatal("fatal msg".into(), "test");

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
    .build_local()
    .unwrap();
  let logger2 = Builder::new()
    .capacity(64)
    .add_flow(MockFlow::new(Level::Trace, out2.clone()))
    .build_local()
    .unwrap();

  logger1.info("from logger1".into(), "test");
  logger2.info("from logger2".into(), "test");

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

  let logger =
    Builder::new().capacity(128).add_flow(flow).build_local().unwrap();

  logger.info("this should be filtered".into(), "test");
  logger.debug("filtered too".into(), "test");
  logger.trace("also filtered".into(), "test");
  logger.warn("this should pass".into(), "test");
  logger.error("error passes too".into(), "test");
  logger.fatal("fatal also passes".into(), "test");

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

  let logger =
    Builder::new().capacity(64).add_flow(flow).build_local().unwrap();

  logger.info("before drop".into(), "test");
  // Drop should not hang
  flush_and_drop(logger);

  assert_eq!(output.lock().unwrap().len(), 1);
}

#[test]
fn test_logger_with_target_routing() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger =
    Builder::new().capacity(128).add_flow(flow).build_local().unwrap();

  logger.info_with_target("targeted log".into(), "test", "mock");
  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 1);
  assert!(logs[0].contains("targeted log"));
}

#[test]
fn test_logger_from_static_variants() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger =
    Builder::new().capacity(128).add_flow(flow).build_local().unwrap();

  logger.trace_from_static("trace static", "test");
  logger.debug_from_static("debug static", "test");
  logger.info_from_static("info static", "test");
  logger.warn_from_static("warn static", "test");
  logger.error_from_static("error static", "test");
  logger.fatal_from_static("fatal static", "test");

  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 6);
}

#[test]
fn test_custom_formatter() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  let logger = Builder::new()
    .capacity(128)
    .format(|mut r: Record| {
      r.content = format!(
        "[{}][{}] {}",
        r.namespace,
        r.level,
        String::from_utf8_lossy(&r.content)
      )
      .into();
      r
    })
    .add_flow(flow)
    .build_local()
    .unwrap();

  logger.info("custom format msg".into(), "myns");
  flush_and_drop(logger);

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 1);
  assert!(logs[0].starts_with("[myns][INFO]"));
}

// ---- Test that uses build() (global installation) ----
// NOTE: Only ONE test in this file can call build() because
// OnceLock can only be set once per process.

#[test]
fn test_global_install_and_macros() {
  let output = Arc::new(Mutex::new(Vec::new()));
  let flow = MockFlow::new(Level::Trace, output.clone());

  // Install globally — this is the only test that does this
  let logger = Builder::new().capacity(256).add_flow(flow).build().unwrap();

  // Test get_instance() works
  let instance = Logger::get_instance();

  // Test all macros (they use get_instance() internally)
  logger::trace!("test", "trace from macro");
  logger::debug!("test", "debug from macro");
  logger::info!("test", "info from macro");
  logger::warn!("test", "warn from macro");
  logger::error!("test", "error from macro");
  logger::fatal!("test", "fatal from macro");

  // Test template macros
  logger::info!("test", "template: {}", "hello");
  logger::info!(target = "mock", "test", "targeted: {}", 42);

  std::thread::sleep(std::time::Duration::from_millis(200));

  let logs = output.lock().unwrap();
  assert_eq!(logs.len(), 8);
  assert!(logs.iter().any(|m| m.contains("trace from macro")));
  assert!(logs.iter().any(|m| m.contains("debug from macro")));
  assert!(logs.iter().any(|m| m.contains("info from macro")));
  assert!(logs.iter().any(|m| m.contains("warn from macro")));
  assert!(logs.iter().any(|m| m.contains("error from macro")));
  assert!(logs.iter().any(|m| m.contains("fatal from macro")));
  assert!(logs.iter().any(|m| m.contains("template: hello")));
  assert!(logs.iter().any(|m| m.contains("targeted: 42")));

  // Clean up
  drop(instance);
  drop(logger);
}
