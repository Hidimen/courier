use std::{io, net::SocketAddr};

/// A provider capability bound to the runtime `R`.
///
/// The runtime is a type parameter, so this crate never names a concrete
/// runtime (such as `depot::Depot`). The composition root instantiates `R`.
///
/// # Examples
///
/// ```rust
/// use std::io;
///
/// use extension::Provider;
///
/// let provider = Provider::new(|_: &mut (), _| -> io::Result<()> { Ok(()) });
/// ```
pub struct Provider<R> {
  pub install: fn(&mut R, SocketAddr) -> io::Result<()>,
}

impl<R> Provider<R> {
  /// Creates a provider executed on `R` by `install`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use std::io;
  ///
  /// use extension::Provider;
  ///
  /// let provider = Provider::new(|_: &mut (), _| -> io::Result<()> { Ok(()) });
  /// ```
  pub fn new(install: fn(&mut R, SocketAddr) -> io::Result<()>) -> Self {
    Self { install }
  }
}
