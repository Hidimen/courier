mod define;

use std::fs::read_to_string;
use std::path::PathBuf;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{LitStr, parse_macro_input, parse_str};

use crate::define::RegistryInput;

fn invocation_dir() -> PathBuf {
  let path = PathBuf::from(Span::call_site().file());

  let path = if path.is_absolute() {
    path
  } else {
    std::env::current_dir()
      .expect("module_register!: failed to get the current working directory")
      .join(path)
  };

  path
    .parent()
    .expect("module_register!: invoking file has no parent directory")
    .to_path_buf()
}

#[proc_macro]
pub fn module_register(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as LitStr);

  let path = invocation_dir().join(input.value());
  let content = read_to_string(&path).unwrap_or_else(|err| {
    panic!("module_register!: failed to read `{}`: {err}", path.display())
  });

  let input = parse_str::<RegistryInput>(&content).unwrap_or_else(|err| {
    panic!("module_register!: failed to parse `{}`: {err}", path.display())
  });

  TokenStream::from(quote! {
    #input
  })
}
