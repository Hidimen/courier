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
  fn println(&mut self, msg: &str) -> Result<(), String> {
    writeln!(self.stdout, "{}", msg).map_err(|e| e.to_string())?;
    Ok(())
  }

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), String> {
    let mut handle = self.stdout.lock();
    for msg in msgs {
      handle.write_all(msg.as_bytes()).map_err(|e| e.to_string())?;
      handle.write_all(b"\n").map_err(|e| e.to_string())?;
    }

    handle.flush().map_err(|e| e.to_string())?;

    Ok(())
  }
}

impl Default for ConsoleFlow {
  fn default() -> Self {
    Self::new()
  }
}
