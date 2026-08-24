use quote::{ToTokens, quote};
use syn::{
  Ident, LitStr, Path, Result, Token, parenthesized, parse::{Parse, ParseStream},
};

pub struct MappingItem {
  key: Ident,
  value: Path,
}

impl Parse for MappingItem {
  fn parse(input: ParseStream) -> Result<Self> {
    let key: Ident = input.parse()?;
    input.parse::<Token![=>]>()?;
    let value: Path = input.parse()?;

    Ok(Self { key, value })
  }
}

impl ToTokens for MappingItem {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let key = &self.key;
    let value = &self.value;

    let feat_name = key.to_token_stream().to_string();

    tokens.extend(quote! {
      #[cfg(feature = #feat_name)]
      #key => #value().await
    });
  }
}

pub struct MacroInput {
  pub list: Vec<MappingItem>,
}

impl Parse for MacroInput {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut list = Vec::new();

    while !input.is_empty() {
      let item: MappingItem = input.parse()?;
      list.push(item);

      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    Ok(Self { list })
  }
}

pub struct ModuleItem {
  ident: Ident,
  kind: String,
  version: String,
}

impl Parse for ModuleItem {
  fn parse(input: ParseStream) -> Result<Self> {
    let ident: Ident = input.parse()?;
    input.parse::<Token![:]>()?;
    let content;
    parenthesized!(content in input);
    let kind: Ident = content.parse()?;
    content.parse::<Token![,]>()?;
    let version: LitStr = content.parse()?;

    Ok(Self { ident, kind: kind.to_string(), version: version.value()})
  }
}

pub struct ModuleInput {
  list: Vec<ModuleItem>,
}

impl Parse for ModuleInput {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut list = Vec::new();

    while !input.is_empty() {
      let item: ModuleItem = input.parse()?;
      list.push(item);

      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    Ok(Self { list })
  }
}

impl ToTokens for ModuleInput {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    for item in &self.list {
      let ident = &item.ident;
      let feat_name = ident.to_token_stream().to_string();

      match item.kind.as_str() {
        "bootloader" => {
          tokens.extend(quote! {
            #[cfg(feature = #feat_name)]
            #[doc(hidden)]
            pub use #ident;
          });
        },
        "full" => {
          tokens.extend(quote! {
            #[cfg(feature = #feat_name)]
            #[doc(inline)]
            pub use #ident;
          });
        },
        "enhancement" => {
          tokens.extend(quote! {
            #[cfg(feature = #feat_name)]
            #[doc(inline)]
            pub use #ident;
          });
        },
        _ => panic!("Unknown kind"),
      }
    }

    let get_all_fn: Vec<_> = self.list.iter().map(|item| {
      let name = item.ident.to_token_stream().to_string();
      let kind = &item.kind;
      let version = &item.version;

      quote! {
        #[cfg(feature = #name)]
        BuiltinModule {
          name: #name,
          version: #version,
          kind: #kind.into()
        }
      }
    }).collect();

    tokens.extend(quote! {
      #[doc(hidden)]
      pub fn get_all_builtins() -> Vec<BuiltinModule> {
        vec![
          #(#get_all_fn),*
        ]
      }
    });
  }
}
