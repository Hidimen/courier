use std::any::Any;

pub trait Event: Any + Send + Sync + 'static {
  fn name(&self) -> &'static str;
}
