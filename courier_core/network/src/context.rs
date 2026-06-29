use crate::Request;

/// Representing processed data provided by specific protocol and pipeline nodes.
///
/// It just serves as a marker trait helping build strong type system.
pub trait Context: Send + Sync + 'static {
  type Request: Request;

  fn request_ref(&self) -> &Self::Request;
}
