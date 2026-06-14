use std::io::{Stdout, Write};

use crate::flow::Flow;

pub struct ConsoleFlow {
  stdout: Stdout,
}

impl ConsoleFlow {
  pub fn new() -> Self {
    Self { stdout: std::io::stdout() }
  }
}

impl Flow for ConsoleFlow {
  fn println(&mut self, msg: &str) -> Result<(), std::io::Error> {
    writeln!(self.stdout, "{}", msg)?;
    Ok(())
  }

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), std::io::Error> {
    let mut handle = self.stdout.lock();
    for msg in msgs {
      handle.write_all(msg.as_bytes())?;
      handle.write_all(b"\n")?;
    }

    handle.flush()?;

    Ok(())
  }

  fn print_bytes(&mut self, msg: &[u8]) -> Result<usize, std::io::Error> {
    self.stdout.write(msg)
  }

  fn flush(&mut self) -> Result<(), std::io::Error> {
    self.stdout.flush()
  }
}

impl Default for ConsoleFlow {
  fn default() -> Self {
    Self::new()
  }
}
