use std::any::{Any, TypeId};

pub trait Event: Any + Send + Sync + 'static {
  fn as_any(&self) -> &dyn Any;

  fn name(&self) -> &'static str;

  fn type_id(&self) -> TypeId {
    TypeId::of::<Self>()
  }
}
