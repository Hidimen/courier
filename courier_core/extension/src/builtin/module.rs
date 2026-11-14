use std::fmt;

use super::capability::*;

/// A builtin module and the capabilities it provides.
///
/// # Examples
///
/// ```rust
/// use extension::Module;
///
/// let module = Module::<()>::new("demo", "0.1.0", "A demo module");
/// assert_eq!(module.name(), "demo");
/// assert!(module.capabilities().is_empty());
/// ```
pub struct Module<R> {
  name: &'static str,
  version: &'static str,
  description: &'static str,
  // Capabilities
  pub stream_provider: Option<Provider<R>>,
  pub datagram_provider: Option<Provider<R>>,
}

impl<R> Module<R> {
  /// Creates a module without capabilities.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::Module;
  ///
  /// let module = Module::<()>::new("demo", "0.1.0", "A demo module");
  /// ```
  pub fn new(
    name: &'static str, version: &'static str, description: &'static str,
  ) -> Self {
    Self {
      name,
      version,
      description,
      stream_provider: None,
      datagram_provider: None,
    }
  }

  pub fn stream_provider(mut self, provider: Provider<R>) -> Self {
    self.stream_provider = Some(provider);
    self
  }

  pub fn datagram_provider(mut self, provider: Provider<R>) -> Self {
    self.datagram_provider = Some(provider);
    self
  }

  /// Returns the name of the module.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::Module;
  ///
  /// assert_eq!(Module::<()>::new("demo", "0.1.0", "A demo module").name(), "demo");
  /// ```
  pub fn name(&self) -> &'static str {
    self.name
  }
}

impl<R> fmt::Display for Module<R> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{} ({}) - {}", self.name, self.version, self.description)
  }
}
