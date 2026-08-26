mod builtin_module;

pub use builtin_module::*;

macro_builtin_build::module_register!("../assets/registry.txt");
