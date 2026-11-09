#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
  PeerClose,
  PeerReset,
  LocalClose,
  Timeout,
  TransportError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseMode {
  Graceful,
  Abort,
}
