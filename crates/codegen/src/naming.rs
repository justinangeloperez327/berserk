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
