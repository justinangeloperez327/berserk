use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Ident, LitStr, Token, Type,
};

#[derive(Clone, Copy)]
enum RelationKind {
    HasMany,
    HasOne,
    BelongsTo,
    BelongsToMany,
}

struct RelationshipArgs {
    related: Type,
    method: Ident,
    options: Vec<(Ident, LitStr)>,
}

impl Parse for RelationshipArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let related = input.parse::<Type>()?;
        input.parse::<Token![,]>()?;

        let method_literal = input.parse::<LitStr>()?;
        let method_name = method_literal.value();
        let method = syn::parse_str::<Ident>(&method_name).map_err(|_| {
            syn::Error::new(
                method_literal.span(),
                "relationship name must be a valid Rust identifier",
            )
        })?;

        let mut options = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let key = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            let value = input.parse::<LitStr>()?;

            if options.iter().any(|(existing, _)| existing == &key) {
                return Err(syn::Error::new(
                    key.span(),
                    format!("duplicate relationship option '{key}'"),
                ));
            }
            if value.value().is_empty() {
                return Err(syn::Error::new(
                    value.span(),
                    format!("relationship option '{key}' cannot be empty"),
                ));
            }
            options.push((key, value));
        }

        Ok(Self {
            related,
            method,
            options,
        })
    }
}

enum RelationSpec {
    HasMany {
        related: Type,
        method: Ident,
        foreign_key: LitStr,
    },
    HasOne {
        related: Type,
        method: Ident,
        foreign_key: LitStr,
    },
    BelongsTo {
        related: Type,
        method: Ident,
        foreign_key: LitStr,
    },
    BelongsToMany {
        related: Type,
        method: Ident,
        pivot: LitStr,
        foreign_pivot_key: LitStr,
        related_pivot_key: LitStr,
    },
}

impl RelationSpec {
    fn method(&self) -> &Ident {
        match self {
            Self::HasMany { method, .. }
            | Self::HasOne { method, .. }
            | Self::BelongsTo { method, .. }
            | Self::BelongsToMany { method, .. } => method,
        }
    }

    fn name(&self) -> LitStr {
        LitStr::new(&self.method().to_string(), self.method().span())
    }

    fn method_tokens(&self) -> TokenStream {
        match self {
            Self::HasMany {
                related,
                method,
                foreign_key,
            } => quote! {
                pub fn #method() -> ::berserk::claw::HasMany<Self, #related> {
                    ::berserk::claw::HasMany::try_new(
                        #foreign_key,
                        |parent: &Self| <Self as ::berserk::claw::Model>::key(parent),
                        |related: &#related| {
                            <#related as ::berserk::claw::Model>::attributes(related)
                                .get(#foreign_key)
                                .cloned()
                                .ok_or_else(|| {
                                    ::berserk::claw::DatabaseError::new(
                                        ::berserk::claw::ErrorKind::Decode,
                                        format!(
                                            "relationship '{}' requires mapped column '{}' on the related model",
                                            stringify!(#method),
                                            #foreign_key
                                        ),
                                    )
                                })
                        },
                    )
                }
            },
            Self::HasOne {
                related,
                method,
                foreign_key,
            } => quote! {
                pub fn #method() -> ::berserk::claw::HasOne<Self, #related> {
                    ::berserk::claw::HasOne::try_new(
                        #foreign_key,
                        |parent: &Self| <Self as ::berserk::claw::Model>::key(parent),
                        |related: &#related| {
                            <#related as ::berserk::claw::Model>::attributes(related)
                                .get(#foreign_key)
                                .cloned()
                                .ok_or_else(|| {
                                    ::berserk::claw::DatabaseError::new(
                                        ::berserk::claw::ErrorKind::Decode,
                                        format!(
                                            "relationship '{}' requires mapped column '{}' on the related model",
                                            stringify!(#method),
                                            #foreign_key
                                        ),
                                    )
                                })
                        },
                    )
                }
            },
            Self::BelongsTo {
                related,
                method,
                foreign_key,
            } => quote! {
                pub fn #method() -> ::berserk::claw::BelongsTo<Self, #related> {
                    ::berserk::claw::BelongsTo::try_new(
                        <#related as ::berserk::claw::Model>::PRIMARY_KEY,
                        |child: &Self| {
                            let value = <Self as ::berserk::claw::Model>::attributes(child)
                                .get(#foreign_key)
                                .cloned()
                                .ok_or_else(|| {
                                    ::berserk::claw::DatabaseError::new(
                                        ::berserk::claw::ErrorKind::Decode,
                                        format!(
                                            "relationship '{}' requires mapped column '{}' on the child model",
                                            stringify!(#method),
                                            #foreign_key
                                        ),
                                    )
                                })?;
                            Ok((value != ::berserk::claw::Value::Null).then_some(value))
                        },
                        |related: &#related| <#related as ::berserk::claw::Model>::key(related),
                    )
                }
            },
            Self::BelongsToMany {
                related,
                method,
                pivot,
                foreign_pivot_key,
                related_pivot_key,
            } => quote! {
                pub fn #method() -> ::berserk::claw::BelongsToMany<Self, #related> {
                    ::berserk::claw::BelongsToMany::new(
                        #pivot,
                        #foreign_pivot_key,
                        #related_pivot_key,
                        |parent: &Self| <Self as ::berserk::claw::Model>::key(parent),
                        |related: &#related| <#related as ::berserk::claw::Model>::key(related),
                    )
                }
            },
        }
    }

    fn loader_arm_tokens(&self) -> TokenStream {
        let name = self.name();
        match self {
            Self::HasMany {
                related, method, ..
            } => quote! {
                #name => {
                    let related = Self::#method().load_on(connection, models)?;
                    Ok(::berserk::claw::NamedRelation::many(
                        #name,
                        related.map(|model| {
                            <#related as ::berserk::claw::Model>::visible_attributes(&model)
                        }),
                    ))
                }
            },
            Self::HasOne {
                related, method, ..
            } => quote! {
                #name => {
                    let related = Self::#method().load_on(connection, models)?;
                    Ok(::berserk::claw::NamedRelation::one(
                        #name,
                        related.map(|model| {
                            <#related as ::berserk::claw::Model>::visible_attributes(&model)
                        }),
                    ))
                }
            },
            Self::BelongsTo {
                method,
                foreign_key,
                ..
            } => quote! {
                #name => {
                    let relation = Self::#method();
                    let related = ::berserk::claw::named_belongs_to(
                        &relation,
                        #foreign_key,
                        connection,
                        models,
                    )?;
                    Ok(::berserk::claw::NamedRelation::one(#name, related))
                }
            },
            Self::BelongsToMany {
                related, method, ..
            } => quote! {
                #name => {
                    let related = Self::#method().load_on(connection, models)?;
                    Ok(::berserk::claw::NamedRelation::many(
                        #name,
                        related.map(|model| {
                            <#related as ::berserk::claw::Model>::visible_attributes(&model)
                        }),
                    ))
                }
            },
        }
    }
}

pub(crate) struct Expansion {
    pub(crate) model_items: TokenStream,
    pub(crate) inherent_impl: TokenStream,
}

pub(crate) fn expand(
    attrs: &[Attribute],
    impl_generics: &syn::ImplGenerics<'_>,
    name: &Ident,
    type_generics: &syn::TypeGenerics<'_>,
    where_clause: Option<&syn::WhereClause>,
) -> syn::Result<Expansion> {
    let relationships = parse(attrs)?;
    if relationships.is_empty() {
        return Ok(Expansion {
            model_items: quote! {},
            inherent_impl: quote! {},
        });
    }

    let methods = relationships.iter().map(RelationSpec::method_tokens);
    let names: Vec<_> = relationships.iter().map(RelationSpec::name).collect();
    let loader_arms = relationships.iter().map(RelationSpec::loader_arm_tokens);

    let model_items = quote! {
        const RELATIONS: &'static [&'static str] = &[#(#names),*];

        fn load_named_relation(
            name: &str,
            connection: &mut dyn ::berserk::claw::Connection,
            models: &[Self],
        ) -> ::berserk::claw::Result<::berserk::claw::NamedRelation> {
            match name {
                #(#loader_arms,)*
                _ => {
                    let available = <Self as ::berserk::claw::Model>::RELATIONS.join(", ");
                    Err(::berserk::claw::DatabaseError::new(
                        ::berserk::claw::ErrorKind::InvalidInput,
                        format!(
                            "unknown relationship '{}' for model '{}'; available: {}",
                            name,
                            <Self as ::berserk::claw::Model>::TABLE,
                            available
                        ),
                    ))
                }
            }
        }
    };

    let inherent_impl = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
    };

    Ok(Expansion {
        model_items,
        inherent_impl,
    })
}

fn parse(attrs: &[Attribute]) -> syn::Result<Vec<RelationSpec>> {
    let mut relationships = Vec::new();
    let mut names = BTreeSet::new();

    for attribute in attrs {
        let kind = if attribute.path().is_ident("has_many") {
            RelationKind::HasMany
        } else if attribute.path().is_ident("has_one") {
            RelationKind::HasOne
        } else if attribute.path().is_ident("belongs_to") {
            RelationKind::BelongsTo
        } else if attribute.path().is_ident("belongs_to_many") {
            RelationKind::BelongsToMany
        } else {
            continue;
        };

        let args = attribute.parse_args::<RelationshipArgs>()?;
        let method_name = args.method.to_string();
        if !names.insert(method_name.clone()) {
            return Err(syn::Error::new(
                args.method.span(),
                format!("duplicate relationship method '{method_name}'"),
            ));
        }

        let spec = match kind {
            RelationKind::HasMany => {
                reject_unknown_options(&args, &["foreign_key"])?;
                RelationSpec::HasMany {
                    foreign_key: required_option(&args, "foreign_key", attribute)?,
                    related: args.related,
                    method: args.method,
                }
            }
            RelationKind::HasOne => {
                reject_unknown_options(&args, &["foreign_key"])?;
                RelationSpec::HasOne {
                    foreign_key: required_option(&args, "foreign_key", attribute)?,
                    related: args.related,
                    method: args.method,
                }
            }
            RelationKind::BelongsTo => {
                reject_unknown_options(&args, &["foreign_key"])?;
                RelationSpec::BelongsTo {
                    foreign_key: required_option(&args, "foreign_key", attribute)?,
                    related: args.related,
                    method: args.method,
                }
            }
            RelationKind::BelongsToMany => {
                reject_unknown_options(
                    &args,
                    &["pivot", "foreign_pivot_key", "related_pivot_key"],
                )?;
                RelationSpec::BelongsToMany {
                    pivot: required_option(&args, "pivot", attribute)?,
                    foreign_pivot_key: required_option(&args, "foreign_pivot_key", attribute)?,
                    related_pivot_key: required_option(&args, "related_pivot_key", attribute)?,
                    related: args.related,
                    method: args.method,
                }
            }
        };

        relationships.push(spec);
    }

    Ok(relationships)
}

fn required_option(
    args: &RelationshipArgs,
    name: &str,
    attribute: &Attribute,
) -> syn::Result<LitStr> {
    args.options
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| {
            syn::Error::new_spanned(
                attribute,
                format!("relationship requires '{name} = \"...\"'"),
            )
        })
}

fn reject_unknown_options(args: &RelationshipArgs, allowed: &[&str]) -> syn::Result<()> {
    for (key, _) in &args.options {
        if !allowed.iter().any(|allowed| key == *allowed) {
            return Err(syn::Error::new(
                key.span(),
                format!("unsupported relationship option '{key}'"),
            ));
        }
    }
    Ok(())
}
