use crate::{Column, DatabaseError, ErrorKind, Result, Row, Statement, Value};
use rusqlite::{params_from_iter, types::ValueRef};

fn parameters(statement: &Statement) -> Result<Vec<rusqlite::types::Value>> {
    statement
        .bindings()
        .iter()
        .map(|value| match value {
            Value::Null => Ok(rusqlite::types::Value::Null),
            Value::Bool(value) => Ok(rusqlite::types::Value::Integer(if *value { 1 } else { 0 })),
            Value::I64(value) => Ok(rusqlite::types::Value::Integer(*value)),
            Value::U64(value) => i64::try_from(*value)
                .map(rusqlite::types::Value::Integer)
                .map_err(|_| {
                    DatabaseError::new(ErrorKind::Decode, "SQLite integers cannot exceed i64::MAX")
                }),
            Value::F64(value) if value.is_finite() => Ok(rusqlite::types::Value::Real(*value)),
            Value::F64(_) => Err(DatabaseError::new(
                ErrorKind::Decode,
                "non-finite floats are not supported",
            )),
            Value::Text(value) => Ok(rusqlite::types::Value::Text(value.clone())),
            Value::Bytes(value) => Ok(rusqlite::types::Value::Blob(value.clone())),
        })
        .collect()
}

pub(super) fn execute(connection: &rusqlite::Connection, statement: &Statement) -> Result<u64> {
    let values = parameters(statement)?;
    connection
        .execute(statement.sql(), params_from_iter(values.iter()))
        .map(|count| count as u64)
        .map_err(super::error::map_error)
}

pub(super) fn query(connection: &rusqlite::Connection, statement: &Statement) -> Result<Vec<Row>> {
    let values = parameters(statement)?;
    let mut prepared = connection
        .prepare(statement.sql())
        .map_err(super::error::map_error)?;
    let names: Vec<String> = prepared
        .column_names()
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let mut source = prepared
        .query(params_from_iter(values.iter()))
        .map_err(super::error::map_error)?;
    let mut output = Vec::new();
    while let Some(row) = source.next().map_err(super::error::map_error)? {
        let mut columns = Vec::with_capacity(names.len());
        for (index, name) in names.iter().enumerate() {
            let value = match row.get_ref(index).map_err(super::error::map_error)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(value) => Value::I64(value),
                ValueRef::Real(value) => Value::F64(value),
                ValueRef::Text(value) => Value::Text(
                    std::str::from_utf8(value)
                        .map_err(|error| DatabaseError::new(ErrorKind::Decode, error.to_string()))?
                        .to_owned(),
                ),
                ValueRef::Blob(value) => Value::Bytes(value.to_vec()),
            };
            columns.push(Column::new(name, value));
        }
        output.push(Row::new(columns)?);
    }
    Ok(output)
}
