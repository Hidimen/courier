use std::time::Duration;

#[derive(Debug)]
pub enum KeepAlive {
  Keep,
  Close,

  UpTo(usize),
  Timeout(Duration),

  Pending,
}
