use crate::Event;

pub struct ServerStart {
  pub time: i64,
}

impl Event for ServerStart {
  fn name(&self) -> &'static str {
    "courier:server_start"
  }
}

pub struct ServerStop {
  pub time: i64,
}

impl Event for ServerStop {
  fn name(&self) -> &'static str {
    "courier:server_stop"
  }
}
