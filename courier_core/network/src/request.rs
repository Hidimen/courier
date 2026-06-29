use std::net::SocketAddr;

pub trait Request: Send + Sync + 'static {
  fn peer_addr(&self) -> Option<SocketAddr>;
  fn local_addr(&self) -> Option<SocketAddr>;
  fn timestamp(&self) -> i64;
}
