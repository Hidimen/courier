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

#[derive(Debug)]
pub struct Logger {
  handle: Mutex<Option<JoinHandle<()>>>,
  sender: Sender<Record>,
  signal: Arc<AtomicBool>,
}

impl Logger {
  pub fn new<F, Formatter>(
    capacity: usize, flows: F, format: Formatter,
  ) -> Arc<Self>
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

    let this = Self { handle: Mutex::new(Some(handle)), sender, signal };

    LOGGER.set(Arc::new(this)).expect("Logger has been initialized");

    LOGGER.get().unwrap().clone()
  }

  pub fn get_instance() -> Arc<Self> {
    match LOGGER.get() {
      Some(s) => s.clone(),
      None => panic!("Logger not initialized"),
    }
  }

  pub fn shutdown(&self) {
    self.signal.store(false, Ordering::Release);
    let _ = self
      .handle
      .lock()
      .unwrap()
      .take()
      .expect("Could not call shutdown twice")
      .join();
  }

  fn log(&self, record: Record) {
    let _ = self.sender.try_send(record);
  }

  pub fn trace(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Trace, namespace));
  }

  pub fn trace_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Trace, namespace, target));
  }

  pub fn trace_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Trace, namespace));
  }

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

  pub fn debug(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Debug, namespace));
  }

  pub fn debug_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Debug, namespace, target));
  }

  pub fn debug_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Debug, namespace));
  }

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

  pub fn info(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Info, namespace));
  }

  pub fn info_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Info, namespace, target));
  }

  pub fn info_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Info, namespace));
  }

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

  pub fn warn(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Warn, namespace));
  }

  pub fn warn_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Warn, namespace, target));
  }

  pub fn warn_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Warn, namespace));
  }

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

  pub fn error(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Error, namespace));
  }

  pub fn error_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Error, namespace, target));
  }

  pub fn error_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Error, namespace));
  }

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

  pub fn fatal(&self, content: String, namespace: &'static str) {
    self.log(Record::new(content, Level::Fatal, namespace));
  }

  pub fn fatal_with_target(
    &self, content: String, namespace: &'static str, target: &'static str,
  ) {
    self.log(Record::new_with_target(content, Level::Fatal, namespace, target));
  }

  pub fn fatal_from_static(
    &self, content: &'static str, namespace: &'static str,
  ) {
    self.log(Record::new_from_static(content, Level::Fatal, namespace));
  }

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

// #[cfg(test)]
// mod tests {
//   use crate::Logger;

//   // #[test]
//   // fn overall() {
//   //   let logger = Logger::new(1000);
//   //   logger.info("test");
//   // }
// }
