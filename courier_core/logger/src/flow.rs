use std::io::Write;

pub trait Flow: Send + 'static {
  fn println(&mut self, msg: &str) -> Result<(), std::io::Error>;

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), std::io::Error>;

  fn print_bytes(&mut self, msg: &[u8]) -> Result<usize, std::io::Error>;

  fn flush(&mut self) -> Result<(), std::io::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn print_batch(&mut self, _msgs: &[&str]) -> Result<(), std::io::Error> {
    Ok(())
  }

  fn println(&mut self, _msg: &str) -> Result<(), std::io::Error> {
    Ok(())
  }

  fn print_bytes(&mut self, _msg: &[u8]) -> Result<usize, std::io::Error> {
    Ok(0)
  }

  fn flush(&mut self) -> Result<(), std::io::Error> {
    Ok(())
  }
}

pub struct Stack<F: Flow, S: Flow> {
  pub first: F,
  pub second: S,
}

impl<F: Flow, S: Flow> Flow for Stack<F, S> {
  fn println(&mut self, msg: &str) -> Result<(), std::io::Error> {
    self.second.println(msg)?;
    self.first.println(msg)
  }

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), std::io::Error> {
    self.second.print_batch(msgs)?;
    self.first.print_batch(msgs)
  }

  fn print_bytes(&mut self, msg: &[u8]) -> Result<usize, std::io::Error> {
    self.second.print_bytes(msg)?;
    self.first.print_bytes(msg)
  }

  fn flush(&mut self) -> Result<(), std::io::Error> {
    self.second.flush()?;
    self.first.flush()
  }
}

impl<F: Flow, S: Flow> Write for Stack<F, S> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.second.print_bytes(buf)?;
    self.first.print_bytes(buf)
  }

  fn flush(&mut self) -> std::io::Result<()> {
    self.second.flush()?;
    self.first.flush()
  }
}
