use std::{io::Write, marker::PhantomData, sync::Arc};

use crate::{
  Logger,
  flow::{Flow, Identity, Stack},
  message::Message,
};

pub type FormatFunction = Box<dyn Fn(&mut dyn Write, Message) + Send>;

pub struct Empty;
pub struct NonEmpty;

pub struct Builder<State, F = ()> {
  capacity: Option<usize>,
  flows: Option<F>,
  format: Option<FormatFunction>,
  _state: PhantomData<State>,
}

impl<State> Builder<State> {
  pub fn new() -> Self {
    Builder { capacity: None, flows: None, format: None, _state: PhantomData }
  }

  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }

  pub fn format<F>(mut self, format: F) -> Self
  where
    F: Fn(&mut dyn Write, Message) + Send + 'static,
  {
    self.format = Some(Box::new(format));
    self
  }
}

impl Builder<Empty> {
  pub fn add_flow<T>(self, flow: T) -> Builder<NonEmpty, Stack<T, Identity>>
  where
    T: Flow + 'static,
  {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { first: flow, second: Identity }),
      format: self.format,
      _state: PhantomData,
    }
  }
}

impl<F> Builder<NonEmpty, F>
where
  F: Flow + Write + 'static,
{
  pub fn add_flow<T>(self, flow: T) -> Builder<NonEmpty, Stack<T, F>>
  where
    T: Flow + 'static,
  {
    Builder {
      capacity: self.capacity,
      flows: Some(Stack { first: flow, second: self.flows.unwrap() }),
      format: self.format,
      _state: PhantomData,
    }
  }

  pub fn build(self) -> Result<Arc<Logger>, &'static str> {
    Ok(Logger::new(
      self.capacity.ok_or("Capacity is unknown")?,
      self.flows.unwrap(),
      self.format.unwrap_or(Box::new(|buf, msg| {
        write!(buf, "[{}][{}] {}\n", msg.timestamp, msg.level, msg.content)
          .unwrap();
      })),
    ))
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
    Builder::new()
      .capacity(1000)
      .add_flow(ConsoleFlow::new())
      .build()
      .unwrap()
      .info("hello");
  }
}
