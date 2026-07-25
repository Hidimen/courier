use std::any::{Any, TypeId};

use crate::Event;

pub trait Subscriber<E: Event>: Any + Send + Sync + 'static {
  fn handle(&self, event: &E);

  fn name(&self) -> &'static str;
}

pub struct Subscription {
  pub type_id: TypeId,
  pub name: &'static str,
}
