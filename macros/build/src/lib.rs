mod define;

use std::fs::read_to_string;
use std::path::PathBuf;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{LitStr, parse_macro_input, parse_str};

use crate::define::{MacroInput, ModuleInput};

fn invocation_dir() -> PathBuf {
  let path = PathBuf::from(Span::call_site().file());

  let path = if path.is_absolute() {
    path
  } else {
    std::env::current_dir()
      .expect(
        "bootloader_register!: failed to get the current working directory",
      )
      .join(path)
  };

  path
    .parent()
    .expect("bootloader_register!: invoking file has no parent directory")
    .to_path_buf()
}

#[proc_macro]
pub fn bootloader_register(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as LitStr);

  let path = invocation_dir().join(input.value());
  let content = read_to_string(&path).unwrap_or_else(|err| {
    panic!("bootloader_register!: failed to read `{}`: {err}", path.display())
  });

  let input = parse_str::<MacroInput>(&content).unwrap_or_else(|err| {
    panic!("bootloader_register!: failed to parse `{}`: {err}", path.display())
  });

  let list = &input.list;

  TokenStream::from(quote! {
    #[doc(hidden)]
    pub async fn run_bootloader<T: ::std::convert::AsRef<::std::primitive::str>>(bootloader: T) -> anyhow::Result<()> {
      match bootloader {
        #(#list,)*
        _ => anyhow::bail!("No bootloader found")
      }
    }
  })
}

#[proc_macro]
pub fn module_register(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as LitStr);

  let path = invocation_dir().join(input.value());
  let content = read_to_string(&path).unwrap_or_else(|err| {
    panic!("module_register!: failed to read `{}`: {err}", path.display())
  });

  let input = parse_str::<ModuleInput>(&content).unwrap_or_else(|err| {
    panic!("module_register!: failed to parse `{}`: {err}", path.display())
  });

  TokenStream::from(quote! {
    #input
  })
}
