use crate::naming::{snake_case, validate_database_name, validate_field_name, validate_type_name};

/// One Rust field emitted into a generated Claw model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSpec {
    name: String,
    rust_type: String,
    column: Option<String>,
    primary_key: bool,
    fillable: bool,
    hidden: bool,
}

impl FieldSpec {
    pub fn new(name: impl Into<String>, rust_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rust_type: rust_type.into(),
            column: None,
            primary_key: false,
            fillable: false,
            hidden: false,
        }
    }

    pub fn string(name: impl Into<String>) -> Self {
        Self::new(name, "String")
    }

    pub fn i64(name: impl Into<String>) -> Self {
        Self::new(name, "i64")
    }

    pub fn u64(name: impl Into<String>) -> Self {
        Self::new(name, "u64")
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        Self::new(name, "bool")
    }

    pub fn column(mut self, column: impl Into<String>) -> Self {
        self.column = Some(column.into());
        self
    }

    pub const fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub const fn fillable(mut self) -> Self {
        self.fillable = true;
        self
    }

    pub const fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rust_type(&self) -> &str {
        &self.rust_type
    }

    pub fn column_name(&self) -> &str {
        self.column.as_deref().unwrap_or(&self.name)
    }

    pub const fn is_primary_key(&self) -> bool {
        self.primary_key
    }

    pub const fn is_fillable(&self) -> bool {
        self.fillable
    }

    pub const fn is_hidden(&self) -> bool {
        self.hidden
    }
}

/// Reusable source-level specification for a Berserk model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSpec {
    name: String,
    table: Option<String>,
    fields: Vec<FieldSpec>,
}

impl ModelSpec {
    /// Create a conventional model with an `i64` `id` primary key.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            table: None,
            fields: vec![FieldSpec::i64("id").primary_key()],
        }
    }

    pub fn table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    pub fn fields(mut self, fields: impl IntoIterator<Item = FieldSpec>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn table_name(&self) -> String {
        self.table
            .clone()
            .unwrap_or_else(|| format!("{}s", snake_case(&self.name)))
    }

    pub fn field_specs(&self) -> &[FieldSpec] {
        &self.fields
    }
}

/// Generate one ordinary Rust model source file.
///
/// This is source scaffolding, not runtime reflection. The generated struct
/// continues to use Berserk's existing `#[derive(Model)]` contract.
pub fn model_source(spec: &ModelSpec) -> syn::Result<String> {
    validate_type_name(spec.name(), "model")?;
    let table = spec.table_name();
    validate_database_name(&table, "table")?;

    if spec.fields.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "model must contain at least one field",
        ));
    }

    let primary_keys = spec
        .fields
        .iter()
        .filter(|field| field.is_primary_key())
        .count();
    if primary_keys != 1 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "model source requires exactly one primary key field",
        ));
    }

    for (index, field) in spec.fields.iter().enumerate() {
        validate_field_name(field.name())?;
        validate_database_name(field.column_name(), "column")?;
        syn::parse_str::<syn::Type>(field.rust_type()).map_err(|_| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("field '{}' must use a valid Rust type", field.name()),
            )
        })?;
        if spec.fields[..index]
            .iter()
            .any(|existing| existing.name() == field.name())
        {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("duplicate model field '{}'", field.name()),
            ));
        }
        if spec.fields[..index]
            .iter()
            .any(|existing| existing.column_name() == field.column_name())
        {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("duplicate model column '{}'", field.column_name()),
            ));
        }
    }

    let mut source = format!(
        "use berserk::Model;\n\n#[derive(Model)]\n#[table(\"{table}\")]\npub struct {} {{\n",
        spec.name()
    );

    for field in &spec.fields {
        if field.is_primary_key() {
            source.push_str("    #[primary_key]\n");
        }
        if field.is_fillable() {
            source.push_str("    #[fillable]\n");
        }
        if field.is_hidden() {
            source.push_str("    #[hidden]\n");
        }
        if field.column_name() != field.name() {
            source.push_str(&format!("    #[column(\"{}\")]\n", field.column_name()));
        }
        source.push_str(&format!(
            "    pub {}: {},\n",
            field.name(),
            field.rust_type()
        ));
        source.push('\n');
    }

    source.push_str("}\n");
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conventional_model_matches_cli_shape() {
        let source = model_source(&ModelSpec::new("User")).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("#[table(\"users\")]"));
        assert!(source.contains("#[primary_key]"));
        assert!(source.contains("pub id: i64"));
    }

    #[test]
    fn model_fields_preserve_claw_metadata() {
        let source = model_source(
            &ModelSpec::new("User")
                .field(FieldSpec::string("name").fillable())
                .field(
                    FieldSpec::string("email")
                        .fillable()
                        .column("email_address"),
                )
                .field(FieldSpec::string("password").fillable().hidden()),
        )
        .unwrap();

        syn::parse_file(&source).unwrap();
        assert!(source.contains("pub name: String"));
        assert!(source.contains("#[column(\"email_address\")]"));
        assert!(source.contains("#[hidden]"));
    }

    #[test]
    fn invalid_models_are_rejected() {
        assert!(model_source(&ModelSpec::new("user")).is_err());
        assert!(model_source(&ModelSpec::new("User").field(FieldSpec::string("id"))).is_err());
        assert!(
            model_source(&ModelSpec::new("User").field(FieldSpec::new("name", "not a type")))
                .is_err()
        );
    }
}
