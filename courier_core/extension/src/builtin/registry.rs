use crate::builtin::Module;

/// Every builtin module compiled into the current binary.
///
/// # Examples
///
/// ```rust
/// use extension::{Module, Registry};
///
/// let mut registry = Registry::<()>::new();
/// registry.register(Module::new("demo", "0.1.0", "A demo module"));
///
/// assert!(registry.get("demo").is_some());
/// assert!(registry.get("missing").is_none());
/// assert_eq!(registry.iter().count(), 1);
/// ```
pub struct Registry<R> {
  modules: Vec<Module<R>>,
}

impl<R> Registry<R> {
  /// Creates an empty registry.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::Registry;
  ///
  /// let registry = Registry::<()>::new();
  /// assert_eq!(registry.iter().count(), 0);
  /// ```
  pub fn new() -> Self {
    Self { modules: Vec::new() }
  }

  /// Registers a module.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::{Module, Registry};
  ///
  /// let mut registry = Registry::<()>::new();
  /// registry.register(Module::new("demo", "0.1.0", "A demo module"));
  /// ```
  pub fn register(&mut self, module: Module<R>) {
    self.modules.push(module);
  }

  /// Returns the module named `name`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::{Module, Registry};
  ///
  /// let mut registry = Registry::<()>::new();
  /// registry.register(Module::new("demo", "0.1.0", "A demo module"));
  ///
  /// assert_eq!(registry.get("demo").map(Module::name), Some("demo"));
  /// ```
  pub fn get(&self, name: &str) -> Option<&Module<R>> {
    self.modules.iter().find(|module| module.name() == name)
  }

  /// Iterates over every registered module.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use extension::{Module, Registry};
  ///
  /// let mut registry = Registry::<()>::new();
  /// registry.register(Module::new("demo", "0.1.0", "A demo module"));
  ///
  /// assert_eq!(registry.iter().map(Module::name).collect::<Vec<_>>(), ["demo"]);
  /// ```
  pub fn iter(&self) -> impl Iterator<Item = &Module<R>> {
    self.modules.iter()
  }
}

impl<R> Default for Registry<R> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn get_finds_the_first_module_with_the_name() {
    let mut registry = Registry::<()>::new();
    registry.register(Module::new("first", "0.1.0", "First"));
    registry.register(Module::new("second", "0.2.0", "Second"));

    assert_eq!(registry.get("second").map(Module::name), Some("second"));
    assert!(registry.get("missing").is_none());
  }

  #[test]
  fn iter_preserves_registration_order() {
    let mut registry = Registry::<()>::new();
    registry.register(Module::new("first", "0.1.0", "First"));
    registry.register(Module::new("second", "0.2.0", "Second"));

    let names: Vec<_> = registry.iter().map(Module::name).collect();
    assert_eq!(names, ["first", "second"]);
  }
}
