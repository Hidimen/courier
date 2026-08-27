use std::fmt::Display;

pub struct BuiltinModule {
  pub name: &'static str,
  pub version: &'static str,
  pub description: &'static str,
}

impl Display for BuiltinModule {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} ({}): {}", self.name, self.version, self.description)
  }
}

#[cfg(test)]
mod tests {
  use crate::BuiltinModule;

  #[test]
  fn fmt() {
    let mock_builtin = BuiltinModule {
      name: "test",
      version: "0.1.0",
      description: "A mock builtin module",
    };

    assert_eq!(mock_builtin.to_string(), "test (0.1.0): A mock builtin module");
  }
}
