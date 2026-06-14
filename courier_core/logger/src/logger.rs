use std::{
  io::Write,
  sync::{Arc, OnceLock},
  thread::JoinHandle,
  time::Duration,
};

use crossbeam::channel::{RecvTimeoutError, Sender, bounded};

use crate::{Level, builder::FormatFunction, flow::Flow, message::Message};

static SINGLETON: OnceLock<Arc<Logger>> = OnceLock::new();

#[derive(Debug)]
pub struct Logger {
  handle: JoinHandle<()>,
  sender: Sender<Message>,
}

impl Logger {
  pub fn new<F>(capacity: usize, flows: F, format: FormatFunction) -> Arc<Self>
  where
    F: Flow + Write,
  {
    let (sender, receiver) = bounded::<Message>(capacity);

    let handle = std::thread::Builder::new()
      .name("logger".into())
      .stack_size(3 * 1024 * 1024)
      .spawn(move || {
        let mut flows = flows;
        let format = format;
        loop {
          match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(data) => {
              (format)(&mut flows, data);
            },
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => continue,
          }
        }
      })
      .expect("Unable to spawn a logger thread");

    let this = Self { handle, sender };

    SINGLETON.set(Arc::new(this)).expect("Logger has been initialized");

    SINGLETON.get().unwrap().clone()
  }

  pub fn get_instance() -> Arc<Self> {
    match SINGLETON.get() {
      Some(s) => s.clone(),
      None => panic!("Logger not initialized"),
    }
  }

  pub fn shutdown(self) {
    drop(self.sender);
    self.handle.join().expect("Failed to join logger thread handle");
  }

  fn log(&self, message: Message) {
    let _ = self.sender.try_send(message);
  }

  pub fn trace(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Trace));
  }

  pub fn debug(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Debug));
  }

  pub fn info(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Info));
  }

  pub fn warn(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Warn));
  }

  pub fn error(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Error));
  }

  pub fn fatal(&self, content: &str) {
    self.log(Message::new(content.to_string(), Level::Fatal));
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
