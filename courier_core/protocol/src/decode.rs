#[must_use = "`Decoded` must use because it may contain data"]
pub enum Decoded<T: Send + 'static> {
  Open(T),
  Closed,
}

unsafe impl<T: Send + 'static> Send for Decoded<T> {}
unsafe impl<T: Send + 'static> Sync for Decoded<T> {}
