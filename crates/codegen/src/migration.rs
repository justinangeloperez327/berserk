use crate::naming::{pascal_case, validate_database_name, validate_migration_name};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationColumnKind {
    String,
    Integer,
    BigInteger,
    Boolean,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationColumnSpec {
    name: String,
    kind: MigrationColumnKind,
    nullable: bool,
    unique: bool,
}

impl MigrationColumnSpec {
    pub fn string(name: impl Into<String>) -> Self {
        Self::new(name, MigrationColumnKind::String)
    }

    pub fn integer(name: impl Into<String>) -> Self {
        Self::new(name, MigrationColumnKind::Integer)
    }

    pub fn big_integer(name: impl Into<String>) -> Self {
        Self::new(name, MigrationColumnKind::BigInteger)
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        Self::new(name, MigrationColumnKind::Boolean)
    }

    fn new(name: impl Into<String>, kind: MigrationColumnKind) -> Self {
        Self {
            name: name.into(),
            kind,
            nullable: false,
            unique: false,
        }
    }

    pub const fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub const fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> MigrationColumnKind {
        self.kind
    }

    fn source(&self) -> String {
        let constructor = match self.kind {
            MigrationColumnKind::String => "string",
            MigrationColumnKind::Integer => "integer",
            MigrationColumnKind::BigInteger => "big_integer",
            MigrationColumnKind::Boolean => "boolean",
        };
        let mut source = format!("Column::{constructor}(\"{}\")", self.name);
        if self.nullable {
            source.push_str(".nullable()");
        }
        if self.unique {
            source.push_str(".unique()");
        }
        source
    }
}

/// Input for Berserk migration source generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationSpec {
    name: String,
    timestamp: u64,
    columns: Vec<MigrationColumnSpec>,
}

impl MigrationSpec {
    pub fn new(name: impl Into<String>, timestamp: u64) -> Self {
        Self {
            name: name.into(),
            timestamp,
            columns: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn column(mut self, column: MigrationColumnSpec) -> Self {
        self.columns.push(column);
        self
    }

    pub fn columns(mut self, columns: impl IntoIterator<Item = MigrationColumnSpec>) -> Self {
        self.columns.extend(columns);
        self
    }

    pub fn column_specs(&self) -> &[MigrationColumnSpec] {
        &self.columns
    }
}

/// Generate one ordinary Rust migration source file.
///
/// Names in the conventional `create_<table>_table` form receive a starter
/// create/drop plan. Other valid migration names receive empty up/down plans
/// that applications can fill with explicit schema operations.
pub fn migration_source(spec: &MigrationSpec) -> syn::Result<String> {
    validate_migration_name(spec.name())?;
    for (index, column) in spec.column_specs().iter().enumerate() {
        validate_database_name(column.name(), "migration column")?;
        if spec.column_specs()[..index]
            .iter()
            .any(|existing| existing.name() == column.name())
        {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("duplicate migration column '{}'", column.name()),
            ));
        }
    }
    let type_name = pascal_case(spec.name());
    let migration_name = format!("{}_{}", spec.timestamp(), spec.name());

    if let Some(table) = spec
        .name()
        .strip_prefix("create_")
        .and_then(|value| value.strip_suffix("_table"))
    {
        let columns = spec
            .column_specs()
            .iter()
            .map(|column| format!("                {},\n", column.source()))
            .collect::<String>();
        return Ok(format!(
            "use berserk::database::{{migrations::{{Column, MigrationPlan, Table}}, Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{\n        \"{migration_name}\"\n    }}\n\n    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .create(Table::create(\"{table}\").columns([\n                Column::id(),\n{columns}                Column::timestamp(\"created_at\"),\n                Column::timestamp(\"updated_at\"),\n            ]))\n            .compile(driver)\n    }}\n\n    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .table(Table::drop(\"{table}\"))\n            .compile(driver)\n    }}\n}}\n"
        ));
    }

    if !spec.column_specs().is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "migration columns require a create_<table>_table migration name",
        ));
    }

    Ok(format!(
        "use berserk::database::{{migrations::MigrationPlan, Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{\n        \"{migration_name}\"\n    }}\n\n    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new().compile(driver)\n    }}\n\n    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new().compile(driver)\n    }}\n}}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_table_migration_is_valid_rust() {
        let source =
            migration_source(&MigrationSpec::new("create_users_table", 1_789_994_000)).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("pub struct CreateUsersTable;"));
        assert!(source.contains("Table::create(\"users\")"));
        assert!(source.contains("Table::drop(\"users\")"));
    }

    #[test]
    fn create_table_migration_accepts_explicit_columns() {
        let source = migration_source(
            &MigrationSpec::new("create_users_table", 1_789_994_000)
                .column(MigrationColumnSpec::string("name"))
                .column(MigrationColumnSpec::boolean("active").nullable()),
        )
        .unwrap();

        syn::parse_file(&source).unwrap();
        assert!(source.contains("Column::string(\"name\")"));
        assert!(source.contains("Column::boolean(\"active\").nullable()"));
    }

    #[test]
    fn generic_migration_is_valid_rust() {
        let source =
            migration_source(&MigrationSpec::new("add_email_to_users", 1_789_994_000)).unwrap();
        syn::parse_file(&source).unwrap();
        assert!(source.contains("pub struct AddEmailToUsers;"));
        assert!(source.contains("MigrationPlan::new().compile(driver)"));
    }

    #[test]
    fn invalid_migration_names_are_rejected() {
        assert!(migration_source(&MigrationSpec::new("CreateUsers", 1)).is_err());
        assert!(migration_source(&MigrationSpec::new("create__users", 1)).is_err());
        assert!(migration_source(&MigrationSpec::new("create_users_", 1)).is_err());
        assert!(migration_source(
            &MigrationSpec::new("create_users_table", 1)
                .column(MigrationColumnSpec::string("bad__column"))
        )
        .is_err());
        assert!(migration_source(
            &MigrationSpec::new("add_email_to_users", 1)
                .column(MigrationColumnSpec::string("email"))
        )
        .is_err());
    }
}
