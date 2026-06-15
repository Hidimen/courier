use crate::Record;

pub trait Format: Send + 'static {
  fn format(&self, record: Record) -> Record;
}

impl<T> Format for T
where
  T: Fn(Record) -> Record + Send + 'static,
{
  fn format(&self, record: Record) -> Record {
    (self)(record)
  }
}
