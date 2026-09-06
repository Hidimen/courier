use std::any::Any;

use crate::Event;

pub struct ServerStart {
  pub time: i64,
}

impl Event for ServerStart {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn name(&self) -> &'static str {
    "courier:server_start"
  }
}

pub struct ServerStop {
  pub time: i64,
}

impl Event for ServerStop {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn name(&self) -> &'static str {
    "courier:server_stop"
  }
}
