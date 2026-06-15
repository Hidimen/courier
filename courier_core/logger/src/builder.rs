use std::{marker::PhantomData, sync::Arc};

use chrono::DateTime;

use crate::{
  Format, Logger, Record,
  flow::{Flow, Identity, Stack},
};

pub struct DefaultFormatter;

impl Format for DefaultFormatter {
  fn format(&self, mut record: Record) -> Record {
    let raw = unsafe { String::from_utf8_unchecked(record.content.into()) };
    let new = format!(
      "[{}][{}] {}",
      DateTime::from_timestamp(record.timestamp, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S"),
      record.level,
      raw
    );
    record.content = new.into();
    record
  }
}

pub struct Empty;
pub struct NonEmpty;

pub struct Builder<State, Formatter = (), F = ()> {
  capacity: Option<usize>,
  flows: Option<F>,
  format: Formatter,
  _state: PhantomData<State>,
}

impl Builder<Empty> {
  pub fn new() -> Builder<Empty, DefaultFormatter> {
    Builder {
      capacity: None,
      flows: None,
      format: DefaultFormatter,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format> Builder<Empty, Formatter> {
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }

  pub fn format<For: Format>(self, formatter: For) -> Builder<Empty, For> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format> Builder<NonEmpty, Formatter> {
  pub fn capacity(mut self, capacity: usize) -> Self {
    self.capacity = Some(capacity);
    self
  }

  pub fn format<For: Format>(self, formatter: For) -> Builder<NonEmpty, For> {
    Builder {
      capacity: self.capacity,
      flows: self.flows,
      format: formatter,
      _state: PhantomData,
    }
  }
}

impl<Formatter: Format> Builder<Empty, Formatter> {
  pub fn add_flow<T>(
    self, flow: T,
  ) -> Builder<NonEmpty, Formatter, Stack<T, Identity>>
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

impl<Formatter: Format, F> Builder<NonEmpty, Formatter, F>
where
  F: Flow + 'static,
{
  pub fn add_flow<T>(self, flow: T) -> Builder<NonEmpty, Formatter, Stack<T, F>>
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
      self.format,
    ))
  }
}
