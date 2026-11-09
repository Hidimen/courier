use std::net::SocketAddr;

use crate::{Payload, TimerId, datagram::close::CloseReason};

pub enum Input<C> {
  Opened,
  Message { peer: SocketAddr, payload: Payload },
  Command(C),
  Timer(TimerId),
  Closed(CloseReason),
}
