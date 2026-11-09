pub trait Protocol: Send + Sized + 'static {
  type Command: Send;
  type Error: std::error::Error + Send + Sync + 'static;
  const NAME: &'static str;
}
