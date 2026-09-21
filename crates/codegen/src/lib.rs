//! Reusable parsing and Rust code generation for Berserk.
//!
//! This crate is intentionally a normal library rather than a proc-macro crate.
//! `berserk-macros` is the compiler-facing procedural-macro shell; this crate
//! owns parsing, validation, and token emission that can also be reused by
//! build-time code generation later.
#![forbid(unsafe_code)]

mod controller;
mod model;
mod relations;

pub use controller::{controller_source, ControllerKind, ControllerSpec};
use proc_macro2::TokenStream;
use syn::DeriveInput;

/// Expand a Berserk `Model` declaration into its Rust implementation.
///
/// The input is ordinary Rust derive input. Syntax failures are returned as
/// `syn::Error` so procedural-macro callers can emit native compiler errors.
pub fn expand_model(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    model::expand(&input)
}
