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

static SINGLETON: OnceLock<Arc<Logger>> = OnceLock::new();

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
            Ok(data) => match flows.println(format.format(data)) {
              Ok(_) => continue,
              Err(HandlingKind::Fuse(reason)) => {
                panic!("Logger encountered a unrecoverable error: {}", reason)
              },
              Err(HandlingKind::Ignore) => continue,
            },
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => continue,
          }
        }

        let _ = flows.flush();
      })
      .expect("Unable to spawn a logger thread");

    let this = Self { handle: Mutex::new(Some(handle)), sender, signal };

    SINGLETON.set(Arc::new(this)).expect("Logger has been initialized");

    SINGLETON.get().unwrap().clone()
  }

  pub fn get_instance() -> Arc<Self> {
    match SINGLETON.get() {
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

  pub fn trace(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Trace));
  }

  pub fn debug(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Debug));
  }

  pub fn info(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Info));
  }

  pub fn warn(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Warn));
  }

  pub fn error(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Error));
  }

  pub fn fatal(&self, content: &str) {
    self.log(Record::new(content.to_string(), Level::Fatal));
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
