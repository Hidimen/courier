use std::{marker::PhantomData, sync::Arc};

use crate::{
  Logger,
  flow::{Flow, Identity, Stack},
};

pub struct Empty;
pub struct NonEmpty;

pub struct Builder<State, F = ()> {
  capacity: Option<usize>,
  flows: Option<F>,
  _state: PhantomData<State>,
}

impl<State> Builder<State> {
  pub fn new() -> Self {
    Builder { capacity: None, flows: None, _state: PhantomData }
  }

  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }
}

impl Builder<Empty> {
  pub fn add_flow<T: Flow>(self, flow: T) -> Builder<NonEmpty, Stack<T, Identity>> {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { first: flow, second: Identity }),
      _state: PhantomData,
    }
  }
}

impl<F: Flow + 'static> Builder<NonEmpty, F> {
  pub fn add_flow<T: Flow>(self, flow: T) -> Builder<NonEmpty, Stack<T, F>> {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { first: flow, second: self.flows.unwrap() }),
      _state: PhantomData,
    }
  }

  pub fn build(self) -> Result<Arc<Logger>, &'static str> {
    Ok(Logger::new(self.capacity.ok_or("Capacity is unknown")?, self.flows.unwrap()))
  }
}

impl<F: Flow> Default for Builder<F> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use crate::{Builder, flows::ConsoleFlow};

  #[test]
  fn overall() {
    Builder::new().capacity(1000).add_flow(ConsoleFlow::new()).build().unwrap().info("hello");
  }
}
