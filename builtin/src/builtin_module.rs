use std::fmt::Display;

pub struct BuiltinModule {
  pub name: &'static str,
  pub version: &'static str,
  pub kind: BuiltinModuleKind
}

impl Display for BuiltinModule {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}, version {}, registered as {}", self.name, self.version, self.kind)
  }
}

pub enum BuiltinModuleKind {
  Bootloader,
  Enhancement,
  Full
}

impl From<&str> for BuiltinModuleKind {
  fn from(value: &str) -> Self {
    match value {
      "bootloader" => Self::Bootloader,
      "enhancement" => Self::Enhancement,
      "full" => Self::Full,
      _ => panic!("Unknown builtin module kind")
    }
  }
}

impl Display for BuiltinModuleKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Bootloader => write!(f, "bootloader"),
      Self::Enhancement => write!(f, "enhancement"),
      Self::Full => write!(f, "full")
    }
  }
}