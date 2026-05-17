pub struct Frame(bytes::Bytes);

impl Frame {
  pub fn new() -> Self {
    Self(bytes::Bytes::new())
  }

  pub fn copy_from_slice(data: &[u8]) -> Self {
    Self(bytes::Bytes::copy_from_slice(data))
  }

  pub const fn from_static(bytes: &'static [u8]) -> Self {
    Self(bytes::Bytes::from_static(bytes))
  }

  pub fn from_owner<T>(owner: T) -> Self
  where
    T: AsRef<[u8]> + Send + 'static,
  {
    Self(bytes::Bytes::from_owner(owner))
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}

impl Default for Frame {
  fn default() -> Self {
    Self::new()
  }
}

impl AsRef<[u8]> for Frame {
  fn as_ref(&self) -> &[u8] {
    self.0.as_ref()
  }
}
