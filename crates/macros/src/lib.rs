//! Compile-time derives for Berserk application code.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod relations;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields, LitStr, Meta, Type,
};

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
    let input = parse_macro_input!(input as DeriveInput);
    match expand_model(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_model(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let table = string_attribute(&input.attrs, "table")?
        .ok_or_else(|| syn::Error::new(input.span(), "Model requires #[table(\"table_name\")]"))?;

    let data = match &input.data {
        Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "Model can only be derived for structs",
            ))
        }
    };
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => {
            return Err(syn::Error::new(
                data.fields.span(),
                "Model requires a struct with named fields",
            ))
        }
    };

    struct FieldSpec {
        ident: syn::Ident,
        ty: Type,
        column: LitStr,
        primary_key: bool,
        fillable: bool,
        hidden: bool,
    }

    let mut specs = Vec::with_capacity(fields.len());
    for field in fields {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new(field.span(), "Model field must be named"))?;
        let column = string_attribute(&field.attrs, "column")?
            .unwrap_or_else(|| LitStr::new(&ident.to_string(), ident.span()));
        specs.push(FieldSpec {
            ident,
            ty: field.ty.clone(),
            column,
            primary_key: marker_attribute(&field.attrs, "primary_key")?,
            fillable: marker_attribute(&field.attrs, "fillable")?,
            hidden: marker_attribute(&field.attrs, "hidden")?,
        });
    }

    let primary: Vec<_> = specs.iter().filter(|field| field.primary_key).collect();
    let primary = match primary.as_slice() {
        [field] => *field,
        [] => {
            return Err(syn::Error::new(
                input.span(),
                "Model requires exactly one #[primary_key] field",
            ))
        }
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "Model cannot have more than one #[primary_key] field",
            ))
        }
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let field_idents: Vec<_> = specs.iter().map(|field| &field.ident).collect();
    let columns: Vec<_> = specs.iter().map(|field| &field.column).collect();
    let fillable: Vec<_> = specs
        .iter()
        .filter(|field| field.fillable)
        .map(|field| &field.column)
        .collect();
    let hidden: Vec<_> = specs
        .iter()
        .filter(|field| field.hidden)
        .map(|field| &field.column)
        .collect();
    let primary_ident = &primary.ident;
    let primary_type = &primary.ty;
    let primary_column = &primary.column;
    let relations::Expansion {
        model_items: relationship_model_items,
        inherent_impl: relationship_impl,
    } = relations::expand(
        &input.attrs,
        &impl_generics,
        name,
        &type_generics,
        where_clause,
    )?;

    Ok(quote! {
        impl #impl_generics ::berserk::claw::Model for #name #type_generics #where_clause {
            const TABLE: &'static str = #table;
            const PRIMARY_KEY: &'static str = #primary_column;
            const FILLABLE: &'static [&'static str] = &[#(#fillable),*];
            const HIDDEN: &'static [&'static str] = &[#(#hidden),*];

            #relationship_model_items

            fn from_row(row: &::berserk::claw::Row) -> ::berserk::claw::Result<Self> {
                Ok(Self {
                    #(#field_idents: ::berserk::claw::field(row, #columns)?,)*
                })
            }

            fn attributes(&self) -> ::berserk::claw::Attributes {
                ::std::collections::BTreeMap::from([
                    #((#columns.to_owned(), ::berserk::claw::Value::from(self.#field_idents.clone())),)*
                ])
            }

            fn key(&self) -> ::berserk::claw::Value {
                ::berserk::claw::Value::from(self.#primary_ident.clone())
            }

            fn parse_route_key(value: &str) -> Option<::berserk::claw::Value> {
                value.parse::<#primary_type>().ok().map(Into::into)
            }
        }

        #relationship_impl
    })
}

fn string_attribute(attrs: &[Attribute], name: &str) -> syn::Result<Option<LitStr>> {
    let mut value = None;
    for attribute in attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident(name))
    {
        if value.is_some() {
            return Err(syn::Error::new_spanned(
                attribute,
                format!("duplicate #[{name}(...)] attribute"),
            ));
        }
        let literal = attribute.parse_args::<LitStr>()?;
        if literal.value().is_empty() {
            return Err(syn::Error::new_spanned(
                attribute,
                format!("#[{name}(...)] cannot be empty"),
            ));
        }
        value = Some(literal);
    }
    Ok(value)
}

fn marker_attribute(attrs: &[Attribute], name: &str) -> syn::Result<bool> {
    let mut found = false;
    for attribute in attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident(name))
    {
        if found {
            return Err(syn::Error::new_spanned(
                attribute,
                format!("duplicate #[{name}] attribute"),
            ));
        }
        if !matches!(&attribute.meta, Meta::Path(_)) {
            return Err(syn::Error::new_spanned(
                attribute,
                format!("#[{name}] does not take arguments"),
            ));
        }
        found = true;
    }
    Ok(found)
}
