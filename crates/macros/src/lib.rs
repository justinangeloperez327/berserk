//! Procedural macro entry points for Berserk application code.
//!
//! Parsing and Rust token generation live in `berserk-codegen` so the same
//! compiler plumbing can be reused by future build-time tooling.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

#[proc_macro_derive(
    Model,
    attributes(
        table,
        primary_key,
        fillable,
        hidden,
        column,
        has_many,
        has_one,
        belongs_to,
        belongs_to_many
    )
)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    match berserk_codegen::expand_model(proc_macro2::TokenStream::from(input)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
