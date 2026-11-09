use crate::{Payload, TimerId, stream::close::CloseReason};

pub enum Input<C> {
  Opened,
  Data(Payload),
  Command(C),
  Timer(TimerId),
  Closed(CloseReason),
}
