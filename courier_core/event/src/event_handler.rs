use std::any::Any;

use crate::{Event, Subscriber};

pub type Handler = Box<dyn Fn(&dyn Any) + Send + Sync + 'static>;

pub struct EventHandler {
  handler: Handler,
  name: &'static str,
}

impl EventHandler {
  pub fn new<S: Subscriber<E>, E: Event>(subscriber: S) -> Self {
    let name = subscriber.name();

    let handler: Handler = Box::new(move |event: &dyn Any| {
      if let Some(e) = event.downcast_ref::<E>() {
        subscriber.handle(e);
      }
    });

    Self { handler, name }
  }

  pub fn execute(&self, event: &dyn Event) {
    (self.handler)(event as &dyn Any)
  }

  pub fn name(&self) -> &'static str {
    self.name
  }
}
