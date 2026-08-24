# Naming style
Names of structs, enums and functions should comply with this convention.

## Crates level
For crate's names, `-` cannot be contained in directories and files for `_` is preferred in Rust.
Also platform-specific rules need your attention.

**Note**: If there exist common components in every crate, like `Builder`, DO NOT add a prefix to it. For instance: `Logger` crate has a builder, which calls `Builder`, not `LoggerBuilder`

## Struct level
Noun is preferred for a single struct. For example, `Logger` and `Depot`.
Struct's name must convey its functionality.

## Function and method level
- `is_*` implies its return value is bool.
- `as_` implies type conversion.
- `into_` implies type conversion but requires ownership.
- `from_*` implies create struct from another value. But attention please, it's better to implement `From` trait before you determine to use `from_*`.
- `new` means creating a struct. If no params required, `default` is a good choice.
- `<VERB> <NOUN>` can be regarded as a best way to describe your function or method.