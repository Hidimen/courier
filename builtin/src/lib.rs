mod builtin_module;

pub use builtin_module::*;

macro_builtin_build::module_register!("../assets/registry.txt");

#[cfg(test)]
mod tests {
  use depot::config::Config;

  /// An unknown bootloader is rejected by the generated `run_instance`.
  #[tokio::test]
  async fn run_instance_rejects_unknown_bootloader() {
    let err = crate::run_instance("no-such-builtin", &Config::default())
      .await
      .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no-such-builtin"), "unexpected error: {msg}");
  }

  /// The generated `get_all_builtins` returns well-formed metadata.
  #[test]
  fn get_all_builtins_returns_registered_metadata() {
    let all = crate::get_all_builtins();
    assert!(all.iter().all(|m| !m.name.is_empty()));
  }
}
