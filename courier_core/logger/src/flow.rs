pub trait Flow: Send + 'static {
  fn println(&mut self, msg: &str) -> Result<(), String>;

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Identity;

impl Flow for Identity {
  fn print_batch(&mut self, _msgs: &[&str]) -> Result<(), String> {
    Ok(())
  }

  fn println(&mut self, _msg: &str) -> Result<(), String> {
    Ok(())
  }
}

pub struct Stack<F: Flow, S: Flow> {
  pub first: F,
  pub second: S,
}

impl<F: Flow, S: Flow> Flow for Stack<F, S> {
  fn println(&mut self, msg: &str) -> Result<(), String> {
    self.second.println(msg)?;
    self.first.println(msg)
  }

  fn print_batch(&mut self, msgs: &[&str]) -> Result<(), String> {
    self.second.print_batch(msgs)?;
    self.first.print_batch(msgs)
  }
}
