use super::{Column, ColumnType, CreateTable};
use crate::{DatabaseError, Driver, ErrorKind, Result, Statement, Value};

pub fn compile_create(table: &CreateTable, driver: Driver) -> Result<Statement> {
    table.validate()?;
    let columns = table
        .column_definitions()
        .iter()
        .map(|column| compile_column(column, driver))
        .collect::<Result<Vec<_>>>()?
        .join(", ");
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

fn compile_default(value: &Value) -> Result<String> {
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
