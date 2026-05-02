pub struct Frame(pub bytes::Bytes);

impl Frame {
  pub fn copy_from_slice(data: &[u8]) -> Self {
    Self(bytes::Bytes::copy_from_slice(data))
  }
}

impl From<&'static [u8]> for Frame {
  fn from(value: &'static [u8]) -> Self {
    Self(bytes::Bytes::from(value))
  }
}
