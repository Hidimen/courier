use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
  Ident, LitStr, Path, Result, Token, braced,
  parse::{Parse, ParseStream},
};

/// The transport kind a builtin module's provider runs over.
///
/// Mirrors the `as stream` / `as datagram` suffix of the `provider` field
/// in `registry.txt` and selects the corresponding provider trait in the
/// depot module.
pub enum ProviderKind {
  /// The provider implements [`depot::StreamProvider`].
  Stream,
  /// The provider implements [`depot::DatagramProvider`].
  Datagram,
}

/// The `provider` field of a module registry entry.
///
/// Carries the provider type path and the transport kind, e.g.
/// `echo::EchoProvider as stream`.
pub struct ProviderField {
  pub path: Path,
  pub kind: ProviderKind,
}

impl Parse for ProviderField {
  fn parse(input: ParseStream) -> Result<Self> {
    let path: Path = input.parse()?;

    let mut kind = ProviderKind::Stream;
    if input.peek(Token![as]) {
      input.parse::<Token![as]>()?;
      let kind_ident: Ident = input.parse()?;
      kind = match kind_ident.to_string().as_str() {
        "stream" => ProviderKind::Stream,
        "datagram" => ProviderKind::Datagram,
        other => {
          return Err(syn::Error::new(
            kind_ident.span(),
            format!(
              "unknown provider kind `{other}`, expected `stream` or \
               `datagram`"
            ),
          ));
        },
      };
    }

    Ok(Self { path, kind })
  }
}

/// One `name => value` entry inside a module block.
///
/// The value is kept as raw tokens. The meaning of a field is interpreted
/// on demand by [`Module`]'s accessors, so the registry is free to grow
/// new fields without the parser rejecting them.
pub struct Field {
  pub name: Ident,
  pub value: TokenStream,
}

/// A single entry in the module registry, e.g.
///
/// ```text
/// echo: {
///   provider => echo::EchoProvider as stream,
/// };
/// ```
///
/// All fields are optional: an entry contributes whatever fields it
/// declares, and only the fields actually present are processed.
pub struct Module {
  pub name: Ident,
  pub fields: Vec<Field>,
}

impl Module {
  /// The module's provider, if it declares one.
  pub fn provider(&self) -> Option<ProviderField> {
    self.typed_field("provider")
  }

  /// The module's version string, if declared.
  pub fn version(&self) -> Option<String> {
    self.typed_field("version").map(|s: LitStr| s.value())
  }

  /// The module's description string, if declared.
  pub fn description(&self) -> Option<String> {
    self.typed_field("description").map(|s: LitStr| s.value())
  }

  /// Parses the raw value of the field named `name` into `T`.
  ///
  /// Returns `None` when the field is absent and panics with a clear
  /// message when it is present but malformed.
  fn typed_field<T: Parse>(&self, name: &str) -> Option<T> {
    self.fields.iter().find(|field| field.name == name).map(|field| {
      syn::parse2(field.value.clone()).unwrap_or_else(|err| {
        panic!(
          "module_register!: module `{}`: invalid `{name}` field: {err}",
          self.name
        )
      })
    })
  }
}

impl Parse for Module {
  fn parse(input: ParseStream) -> Result<Self> {
    let name: Ident = input.parse()?;
    input.parse::<Token![:]>()?;

    let content;
    braced!(content in input);

    let mut fields = Vec::new();

    while !content.is_empty() {
      let field: Ident = content.parse()?;
      content.parse::<Token![=>]>()?;
      let value = parse_field_value(&content)?;
      fields.push(Field { name: field, value });

      if content.peek(Token![,]) {
        content.parse::<Token![,]>()?;
      }
    }

    Ok(Self { name, fields })
  }
}

/// Collects the tokens of a field value, stopping at the next top-level
/// `,` or the end of the module block.
fn parse_field_value(input: ParseStream) -> Result<TokenStream> {
  let mut tokens = TokenStream::new();

  while !input.is_empty() && !input.peek(Token![,]) {
    let tree: TokenTree = input.parse()?;
    tokens.extend([tree]);
  }

  Ok(tokens)
}

/// The whole `registry.txt` file: a list of [`Module`] entries.
pub struct RegistryInput {
  pub modules: Vec<Module>,
}

impl Parse for RegistryInput {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut modules = Vec::new();

    while !input.is_empty() {
      modules.push(input.parse()?);

      // Entries may be separated by `,` or terminated by a trailing `;`.
      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
      if input.peek(Token![;]) {
        input.parse::<Token![;]>()?;
      }
    }

    Ok(Self { modules })
  }
}

impl ToTokens for RegistryInput {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let run_arms: Vec<_> = self
      .modules
      .iter()
      .filter_map(|module| {
        let provider = module.provider()?;
        let name: String = module.name.to_string();
        let path = provider.path;
        let run_kind = match provider.kind {
          ProviderKind::Stream => quote! {
            use depot::StreamProvider;
            depot.spawn_stream(provider, address, pod_name.clone())?
          },
          ProviderKind::Datagram => quote! {
            use depot::DatagramProvider;
            depot.spawn_datagram(provider, address, pod_name.clone())?
          },
        };

        Some(quote! {
          #name => {
            let provider = #path::create();
            #run_kind
          }
        })
      })
      .collect();

    let exports: Vec<_> = self
      .modules
      .iter()
      .map(|module| {
        let ident = &module.name;
        quote! {
          pub use #ident;
        }
      })
      .collect();

    let builtins: Vec<_> = self
      .modules
      .iter()
      .map(|module| {
        let name = module.name.to_string();
        let version = module.version().unwrap_or_default();
        let description = module.description().unwrap_or_default();

        quote! {
          BuiltinModule {
            name: #name,
            version: #version,
            description: #description,
          }
        }
      })
      .collect();

    tokens.extend(quote! {
      #(#exports)*
    });

    tokens.extend(quote! {
      #[doc(hidden)]
      pub async fn run(
        config: &depot::config::Config,
      ) -> anyhow::Result<depot::Depot> {
        let mut depot = depot::Depot::new(config.shutdown_timeout);
        for pod in &config.pods {
          let pod_name = pod.name.clone().unwrap_or("Unnamed".to_string());
          for addr in &pod.listen {
            let address = addr.parse::<std::net::SocketAddr>()?;
            let provider_name = if let Some(name) = &pod.provider {
              name
            }else {
              anyhow::bail!("No provider specified");
            };
            match provider_name.as_str() {
              #(#run_arms,)*
              other => anyhow::bail!("No provider named `{other}` is registered"),
            };
          }
        }

        Ok(depot)
      }

      /// Returns the metadata of every builtin module in `registry.txt`.
      #[doc(hidden)]
      pub fn get_all_builtins() -> Vec<BuiltinModule> {
        vec![
          #(#builtins,)*
        ]
      }
    });
  }
}
