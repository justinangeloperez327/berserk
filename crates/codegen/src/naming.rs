use proc_macro2::Span;

pub(crate) fn validate_type_name(name: &str, kind: &str) -> syn::Result<()> {
    let valid_shape = name.bytes().enumerate().all(|(index, byte)| {
        byte.is_ascii_alphabetic() && (index > 0 || byte.is_ascii_uppercase())
            || byte.is_ascii_digit() && index > 0
    });
    if name.is_empty() || name.len() > 64 || !valid_shape {
        return Err(syn::Error::new(
            Span::call_site(),
            format!("{kind} name must be a PascalCase Rust identifier"),
        ));
    }

    syn::parse_str::<syn::Ident>(name).map(|_| ()).map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            format!("{kind} name must be a valid Rust identifier"),
        )
    })
}

pub(crate) fn snake_case(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}

pub(crate) fn validate_migration_name(name: &str) -> syn::Result<()> {
    let valid = !name.is_empty()
        && name.len() <= 96
        && name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'_' && index > 0
        })
        && !name.ends_with('_')
        && !name.contains("__");

    if valid {
        Ok(())
    } else {
        Err(syn::Error::new(
            Span::call_site(),
            "migration name must be snake_case without repeated or edge underscores",
        ))
    }
}

pub(crate) fn pascal_case(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

pub(crate) fn validate_field_name(name: &str) -> syn::Result<()> {
    let valid_shape = !name.is_empty()
        && name.len() <= 64
        && name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'_' && index > 0
        })
        && !name.ends_with('_')
        && !name.contains("__");

    if !valid_shape || syn::parse_str::<syn::Ident>(name).is_err() {
        return Err(syn::Error::new(
            Span::call_site(),
            "model field name must be a snake_case Rust identifier",
        ));
    }
    Ok(())
}

pub(crate) fn validate_database_name(name: &str, kind: &str) -> syn::Result<()> {
    let valid = !name.is_empty()
        && name.len() <= 63
        && name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'_' && index > 0
        })
        && !name.ends_with('_')
        && !name.contains("__");

    if valid {
        Ok(())
    } else {
        Err(syn::Error::new(
            Span::call_site(),
            format!("{kind} name must be portable lowercase snake_case"),
        ))
    }
}
