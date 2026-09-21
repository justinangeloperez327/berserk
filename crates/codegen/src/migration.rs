use crate::naming::{pascal_case, validate_migration_name};

/// Input for Berserk migration source generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationSpec {
    name: String,
    timestamp: u64,
}

impl MigrationSpec {
    pub fn new(name: impl Into<String>, timestamp: u64) -> Self {
        Self {
            name: name.into(),
            timestamp,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Generate one ordinary Rust migration source file.
///
/// Names in the conventional `create_<table>_table` form receive a starter
/// create/drop plan. Other valid migration names receive empty up/down plans
/// that applications can fill with explicit schema operations.
pub fn migration_source(spec: &MigrationSpec) -> syn::Result<String> {
    validate_migration_name(spec.name())?;
    let type_name = pascal_case(spec.name());
    let migration_name = format!("{}_{}", spec.timestamp(), spec.name());

    if let Some(table) = spec
        .name()
        .strip_prefix("create_")
        .and_then(|value| value.strip_suffix("_table"))
    {
        return Ok(format!(
            "use berserk::database::{{migrations::{{Column, MigrationPlan, Table}}, Driver, Migration, Result, Statement}};\n\npub struct {type_name};\n\nimpl Migration for {type_name} {{\n    fn name(&self) -> &'static str {{\n        \"{migration_name}\"\n    }}\n\n    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .create(Table::create(\"{table}\").columns([\n                Column::id(),\n                Column::timestamp(\"created_at\"),\n                Column::timestamp(\"updated_at\"),\n            ]))\n            .compile(driver)\n    }}\n\n    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {{\n        MigrationPlan::new()\n            .table(Table::drop(\"{table}\"))\n            .compile(driver)\n    }}\n}}\n"
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
    }
}
