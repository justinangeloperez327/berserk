use super::{
    AlterOperation, AlterTable, Column, ColumnDefault, ColumnType, CreateTable, ForeignAction,
    RebuildTable, TableOperation,
};
use crate::{DatabaseError, Driver, ErrorKind, Result, Statement, Value};

pub fn compile_create(table: &CreateTable, driver: Driver) -> Result<Statement> {
    table.validate()?;
    let columns = table
        .column_definitions()
        .iter()
        .map(|column| compile_column(column, driver))
        .collect::<Result<Vec<_>>>()?;
    let mut definitions = columns;
    if let Some(primary_key) = &table.primary_key {
        let columns = primary_key
            .iter()
            .map(|column| quote_identifier(column, driver))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        definitions.push(format!("PRIMARY KEY ({columns})"));
    }
    for unique in &table.uniques {
        let columns = unique
            .columns
            .iter()
            .map(|column| quote_identifier(column, driver))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let constraint = match &unique.name {
            Some(name) => format!("CONSTRAINT {} ", quote_identifier(name, driver)?),
            None => String::new(),
        };
        definitions.push(format!("{constraint}UNIQUE ({columns})"));
    }
    for check in &table.checks {
        definitions.push(format!(
            "CONSTRAINT {} CHECK ({})",
            quote_identifier(&check.name, driver)?,
            check.expression
        ));
    }
    for foreign_key in &table.foreign_keys {
        let local = foreign_key
            .columns
            .iter()
            .map(|column| quote_identifier(column, driver))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let referenced = foreign_key
            .referenced_columns
            .iter()
            .map(|column| quote_identifier(column, driver))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let constraint = match &foreign_key.name {
            Some(name) => format!("CONSTRAINT {} ", quote_identifier(name, driver)?),
            None => String::new(),
        };
        let mut definition = format!(
            "{constraint}FOREIGN KEY ({local}) REFERENCES {} ({referenced})",
            quote_identifier(&foreign_key.referenced_table, driver)?
        );
        if let Some(action) = foreign_key.on_delete {
            definition.push_str(" ON DELETE ");
            definition.push_str(compile_foreign_action(action));
        }
        if let Some(action) = foreign_key.on_update {
            definition.push_str(" ON UPDATE ");
            definition.push_str(compile_foreign_action(action));
        }
        definitions.push(definition);
    }
    let columns = definitions.join(", ");
    Ok(Statement::new(format!(
        "CREATE TABLE {} ({columns})",
        quote_identifier(table.name(), driver)?
    )))
}

fn compile_column(column: &Column, driver: Driver) -> Result<String> {
    let mut sql = format!(
        "{} {}",
        quote_identifier(column.name(), driver)?,
        compile_type(column, driver)?
    );
    if column.primary {
        sql.push_str(" PRIMARY KEY");
    }
    if !column.nullable && !matches!(column.kind, ColumnType::Id) {
        sql.push_str(" NOT NULL");
    }
    if column.unique {
        sql.push_str(" UNIQUE");
    }
    if let Some(default) = &column.default {
        sql.push_str(" DEFAULT ");
        sql.push_str(&compile_default(default)?);
    }
    Ok(sql)
}

fn compile_type(column: &Column, driver: Driver) -> Result<String> {
    let sql = match &column.kind {
        ColumnType::Id => match driver {
            Driver::Postgres => "BIGSERIAL".into(),
            Driver::MySql => "BIGINT UNSIGNED AUTO_INCREMENT".into(),
            Driver::Sqlite => "INTEGER".into(),
        },
        ColumnType::String(length) => format!("VARCHAR({})", length.unwrap_or(255)),
        ColumnType::Text => "TEXT".into(),
        ColumnType::TinyInteger => match driver {
            Driver::Postgres => "SMALLINT".into(),
            Driver::MySql => "TINYINT".into(),
            Driver::Sqlite => "INTEGER".into(),
        },
        ColumnType::SmallInteger => "SMALLINT".into(),
        ColumnType::Integer => "INTEGER".into(),
        ColumnType::BigInteger => "BIGINT".into(),
        ColumnType::Decimal { precision, scale } => {
            format!("DECIMAL({precision},{scale})")
        }
        ColumnType::Float => match driver {
            Driver::Postgres => "REAL".into(),
            Driver::MySql => "FLOAT".into(),
            Driver::Sqlite => "REAL".into(),
        },
        ColumnType::Double => match driver {
            Driver::Postgres => "DOUBLE PRECISION".into(),
            Driver::MySql => "DOUBLE".into(),
            Driver::Sqlite => "REAL".into(),
        },
        ColumnType::Boolean => match driver {
            Driver::MySql => "BOOLEAN".into(),
            Driver::Postgres | Driver::Sqlite => "BOOLEAN".into(),
        },
        ColumnType::Date => "DATE".into(),
        ColumnType::Time => "TIME".into(),
        ColumnType::DateTime => match driver {
            Driver::Postgres => "TIMESTAMP".into(),
            Driver::MySql => "DATETIME".into(),
            Driver::Sqlite => "TEXT".into(),
        },
        ColumnType::Timestamp => match driver {
            Driver::Sqlite => "TEXT".into(),
            Driver::Postgres | Driver::MySql => "TIMESTAMP".into(),
        },
        ColumnType::Json => match driver {
            Driver::Postgres => "JSONB".into(),
            Driver::MySql => "JSON".into(),
            Driver::Sqlite => "TEXT".into(),
        },
        ColumnType::Binary => match driver {
            Driver::Postgres => "BYTEA".into(),
            Driver::MySql => "BLOB".into(),
            Driver::Sqlite => "BLOB".into(),
        },
        ColumnType::Uuid => match driver {
            Driver::Postgres => "UUID".into(),
            Driver::MySql => "CHAR(36)".into(),
            Driver::Sqlite => "TEXT".into(),
        },
    };
    Ok(sql)
}

pub fn compile_indexes(table: &CreateTable, driver: Driver) -> Result<Vec<Statement>> {
    table.validate()?;
    table
        .indexes
        .iter()
        .map(|index| {
            let columns = index
                .columns
                .iter()
                .map(|column| quote_identifier(column, driver))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            let generated_name = format!("idx_{}_{}", table.name, index.columns.join("_"));
            let name = index.name.as_deref().unwrap_or(&generated_name);
            let unique = if index.unique { "UNIQUE " } else { "" };
            Ok(Statement::new(format!(
                "CREATE {unique}INDEX {} ON {} ({columns})",
                quote_identifier(name, driver)?,
                quote_identifier(&table.name, driver)?
            )))
        })
        .collect()
}

fn compile_foreign_action(action: ForeignAction) -> &'static str {
    match action {
        ForeignAction::Cascade => "CASCADE",
        ForeignAction::Restrict => "RESTRICT",
        ForeignAction::SetNull => "SET NULL",
        ForeignAction::NoAction => "NO ACTION",
    }
}

fn compile_default(default: &ColumnDefault) -> Result<String> {
    let ColumnDefault::Value(value) = default else {
        return Ok("CURRENT_TIMESTAMP".into());
    };
    match value {
        Value::Null => Ok("NULL".into()),
        Value::Bool(value) => Ok(if *value { "TRUE" } else { "FALSE" }.into()),
        Value::I64(value) => Ok(value.to_string()),
        Value::U64(value) => Ok(value.to_string()),
        Value::F64(value) if value.is_finite() => Ok(value.to_string()),
        Value::F64(_) => Err(error("non-finite migration defaults are not supported")),
        Value::Text(value) => Ok(format!("'{}'", value.replace('\'', "''"))),
        Value::Bytes(_) => Err(error("binary migration defaults are not supported")),
    }
}

fn quote_identifier(identifier: &str, driver: Driver) -> Result<String> {
    if identifier.is_empty()
        || identifier.len() > 63
        || !identifier
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        || !identifier
            .chars()
            .next()
            .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
    {
        return Err(error(format!("invalid SQL identifier `{identifier}`")));
    }
    Ok(match driver {
        Driver::MySql => format!("`{identifier}`"),
        Driver::Postgres | Driver::Sqlite => format!("\"{identifier}\""),
    })
}

fn error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Query, message)
}

pub fn compile_alter(table: &AlterTable, driver: Driver) -> Result<Vec<Statement>> {
    table.validate()?;
    let table_name = quote_identifier(&table.name, driver)?;
    let mut statements = Vec::new();

    for operation in &table.operations {
        match operation {
            AlterOperation::Add(column) => statements.push(Statement::new(format!(
                "ALTER TABLE {table_name} ADD COLUMN {}",
                compile_column(column, driver)?
            ))),
            AlterOperation::Drop(column) => statements.push(Statement::new(format!(
                "ALTER TABLE {table_name} DROP COLUMN {}",
                quote_identifier(column, driver)?
            ))),
            AlterOperation::Rename { from, to } => statements.push(Statement::new(format!(
                "ALTER TABLE {table_name} RENAME COLUMN {} TO {}",
                quote_identifier(from, driver)?,
                quote_identifier(to, driver)?
            ))),
            AlterOperation::Modify(column) => match driver {
                Driver::Postgres => {
                    let column_name = quote_identifier(column.name(), driver)?;
                    statements.push(Statement::new(format!(
                        "ALTER TABLE {table_name} ALTER COLUMN {column_name} TYPE {}",
                        compile_type(column, driver)?
                    )));
                    statements.push(Statement::new(format!(
                        "ALTER TABLE {table_name} ALTER COLUMN {column_name} {} NOT NULL",
                        if column.nullable { "DROP" } else { "SET" }
                    )));
                    if let Some(default) = &column.default {
                        statements.push(Statement::new(format!(
                            "ALTER TABLE {table_name} ALTER COLUMN {column_name} SET DEFAULT {}",
                            compile_default(default)?
                        )));
                    }
                }
                Driver::MySql => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} MODIFY COLUMN {}",
                    compile_column(column, driver)?
                ))),
                Driver::Sqlite => {
                    return Err(error(
                        "SQLite column modification requires an explicit table rebuild migration",
                    ));
                }
            },
            AlterOperation::AddIndex(index) => {
                let columns = index
                    .columns
                    .iter()
                    .map(|column| quote_identifier(column, driver))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let generated_name = format!("idx_{}_{}", table.name, index.columns.join("_"));
                let name = index.name.as_deref().unwrap_or(&generated_name);
                let unique = if index.unique { "UNIQUE " } else { "" };
                statements.push(Statement::new(format!(
                    "CREATE {unique}INDEX {} ON {table_name} ({columns})",
                    quote_identifier(name, driver)?
                )));
            }
            AlterOperation::DropIndex(name) => statements.push(Statement::new(match driver {
                Driver::MySql => format!(
                    "DROP INDEX {} ON {table_name}",
                    quote_identifier(name, driver)?
                ),
                Driver::Postgres | Driver::Sqlite => {
                    format!("DROP INDEX {}", quote_identifier(name, driver)?)
                }
            })),
            AlterOperation::RenameIndex { from, to } => match driver {
                Driver::Postgres => statements.push(Statement::new(format!(
                    "ALTER INDEX {} RENAME TO {}",
                    quote_identifier(from, driver)?,
                    quote_identifier(to, driver)?
                ))),
                Driver::MySql => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} RENAME INDEX {} TO {}",
                    quote_identifier(from, driver)?,
                    quote_identifier(to, driver)?
                ))),
                Driver::Sqlite => {
                    return Err(error(
                        "SQLite does not support renaming indexes; drop and recreate the index",
                    ));
                }
            },
            AlterOperation::AddForeignKey(foreign_key) => {
                if matches!(driver, Driver::Sqlite) {
                    return Err(error(
                        "SQLite foreign key changes require an explicit table rebuild migration",
                    ));
                }
                let local = foreign_key
                    .columns
                    .iter()
                    .map(|column| quote_identifier(column, driver))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let referenced = foreign_key
                    .referenced_columns
                    .iter()
                    .map(|column| quote_identifier(column, driver))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let constraint = match &foreign_key.name {
                    Some(name) => format!("CONSTRAINT {} ", quote_identifier(name, driver)?),
                    None => String::new(),
                };
                let mut clause = format!(
                    "ALTER TABLE {table_name} ADD {constraint}FOREIGN KEY ({local}) REFERENCES {} ({referenced})",
                    quote_identifier(&foreign_key.referenced_table, driver)?
                );
                if let Some(action) = foreign_key.on_delete {
                    clause.push_str(" ON DELETE ");
                    clause.push_str(compile_foreign_action(action));
                }
                if let Some(action) = foreign_key.on_update {
                    clause.push_str(" ON UPDATE ");
                    clause.push_str(compile_foreign_action(action));
                }
                statements.push(Statement::new(clause));
            }
            AlterOperation::DropForeignKey(name) => match driver {
                Driver::Postgres => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP CONSTRAINT {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::MySql => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP FOREIGN KEY {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::Sqlite => {
                    return Err(error(
                        "SQLite foreign key changes require an explicit table rebuild migration",
                    ));
                }
            },
            AlterOperation::SetDefault { column, default } => {
                if matches!(driver, Driver::Sqlite) {
                    return Err(error(
                        "SQLite default changes require an explicit table rebuild migration",
                    ));
                }
                statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} ALTER COLUMN {} SET DEFAULT {}",
                    quote_identifier(column, driver)?,
                    compile_default(default)?
                )));
            }
            AlterOperation::DropDefault(column) => {
                if matches!(driver, Driver::Sqlite) {
                    return Err(error(
                        "SQLite default changes require an explicit table rebuild migration",
                    ));
                }
                statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} ALTER COLUMN {} DROP DEFAULT",
                    quote_identifier(column, driver)?
                )));
            }
            AlterOperation::AddCheck(check) => {
                if matches!(driver, Driver::Sqlite) {
                    return Err(error(
                        "SQLite check constraint changes require an explicit table rebuild migration",
                    ));
                }
                statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} ADD CONSTRAINT {} CHECK ({})",
                    quote_identifier(&check.name, driver)?,
                    check.expression
                )));
            }
            AlterOperation::DropCheck(name) => match driver {
                Driver::Postgres => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP CONSTRAINT {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::MySql => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP CHECK {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::Sqlite => {
                    return Err(error(
                        "SQLite check constraint changes require an explicit table rebuild migration",
                    ));
                }
            },
            AlterOperation::AddUnique(unique) => {
                if matches!(driver, Driver::Sqlite) {
                    return Err(error(
                        "SQLite unique constraint changes require an explicit table rebuild migration",
                    ));
                }
                let columns = unique
                    .columns
                    .iter()
                    .map(|column| quote_identifier(column, driver))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                let name = unique.name.as_ref().ok_or_else(|| {
                    error("alter-table unique constraints must have an explicit name")
                })?;
                statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} ADD CONSTRAINT {} UNIQUE ({columns})",
                    quote_identifier(name, driver)?
                )));
            }
            AlterOperation::DropUnique(name) => match driver {
                Driver::Postgres => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP CONSTRAINT {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::MySql => statements.push(Statement::new(format!(
                    "ALTER TABLE {table_name} DROP INDEX {}",
                    quote_identifier(name, driver)?
                ))),
                Driver::Sqlite => {
                    return Err(error(
                        "SQLite unique constraint changes require an explicit table rebuild migration",
                    ));
                }
            },
        }
    }

    Ok(statements)
}


pub fn compile_rebuild(rebuild: &RebuildTable, driver: Driver) -> Result<Vec<Statement>> {
    rebuild.validate()?;
    if !matches!(driver, Driver::Sqlite) {
        return Err(error("table rebuild migrations are only supported for SQLite"));
    }

    let temporary_name = format!("__br_{}", rebuild.name);
    let mut temporary = rebuild.replacement.clone();
    temporary.name = temporary_name.clone();
    temporary.indexes.clear();

    let mut statements = vec![compile_create(&temporary, driver)?];
    let source_columns = rebuild
        .copy
        .iter()
        .map(|(from, _)| quote_identifier(from, driver))
        .collect::<Result<Vec<_>>>()?
        .join(", ");
    let target_columns = rebuild
        .copy
        .iter()
        .map(|(_, to)| quote_identifier(to, driver))
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    statements.push(Statement::new(format!(
        "INSERT INTO {} ({target_columns}) SELECT {source_columns} FROM {}",
        quote_identifier(&temporary_name, driver)?,
        quote_identifier(&rebuild.name, driver)?
    )));
    statements.push(compile_table_operation(
        &TableOperation::Drop {
            name: rebuild.name.clone(),
            if_exists: false,
        },
        driver,
    )?);
    statements.push(compile_table_operation(
        &TableOperation::Rename {
            from: temporary_name,
            to: rebuild.name.clone(),
        },
        driver,
    )?);
    statements.extend(compile_indexes(&rebuild.replacement, driver)?);
    Ok(statements)
}

pub fn compile_table_operation(operation: &TableOperation, driver: Driver) -> Result<Statement> {
    operation.validate()?;
    match operation {
        TableOperation::Rename { from, to } => Ok(Statement::new(format!(
            "ALTER TABLE {} RENAME TO {}",
            quote_identifier(from, driver)?,
            quote_identifier(to, driver)?
        ))),
        TableOperation::Drop { name, if_exists } => Ok(Statement::new(format!(
            "DROP TABLE {}{}",
            if *if_exists { "IF EXISTS " } else { "" },
            quote_identifier(name, driver)?
        ))),
    }
}
