#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
  LocalClose,
  Timeout,
  TransportError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseMode {
  Graceful,
  Abort,
}
