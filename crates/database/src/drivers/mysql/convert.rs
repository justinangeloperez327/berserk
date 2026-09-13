use crate::{Column, DatabaseError, ErrorKind, Result, Row, Statement, Value};
use mysql::{
    consts::{ColumnFlags, ColumnType},
    prelude::Queryable,
    Params,
};

fn parameters(statement: &Statement) -> Result<Params> {
    let mut values = Vec::with_capacity(statement.bindings().len());
    for value in statement.bindings() {
        values.push(match value {
            Value::Null => mysql::Value::NULL,
            Value::Bool(value) => mysql::Value::Int(if *value { 1 } else { 0 }),
            Value::I64(value) => mysql::Value::Int(*value),
            Value::U64(value) => mysql::Value::UInt(*value),
            Value::F64(value) if value.is_finite() => mysql::Value::Double(*value),
            Value::F64(_) => {
                return Err(DatabaseError::new(
                    ErrorKind::Decode,
                    "non-finite floats are not supported",
                ))
            }
            Value::Text(value) => mysql::Value::Bytes(value.as_bytes().to_vec()),
            Value::Bytes(value) => mysql::Value::Bytes(value.clone()),
        });
    }
    Ok(Params::Positional(values))
}

pub(super) fn execute(client: &mut impl Queryable, statement: &Statement) -> Result<()> {
    client
        .exec_drop(statement.sql(), parameters(statement)?)
        .map_err(super::error::map_error)
}

pub(super) fn query(client: &mut impl Queryable, statement: &Statement) -> Result<Vec<Row>> {
    let rows: Vec<mysql::Row> = client
        .exec(statement.sql(), parameters(statement)?)
        .map_err(super::error::map_error)?;
    rows.into_iter().map(convert_row).collect()
}

fn convert_row(row: mysql::Row) -> Result<Row> {
    let metadata: Vec<(String, ColumnType, ColumnFlags)> = row
        .columns_ref()
        .iter()
        .map(|column| {
            (
                column.name_str().into_owned(),
                column.column_type(),
                column.flags(),
            )
        })
        .collect();
    let values = row.unwrap();
    let columns = metadata
        .into_iter()
        .zip(values)
        .map(|((name, ty, flags), value)| {
            convert_value(value, ty, flags).map(|value| Column::new(name, value))
        })
        .collect::<Result<Vec<_>>>()?;
    Row::new(columns)
}

fn convert_value(value: mysql::Value, ty: ColumnType, flags: ColumnFlags) -> Result<Value> {
    match value {
        mysql::Value::NULL => Ok(Value::Null),
        mysql::Value::Int(value) => Ok(Value::I64(value)),
        mysql::Value::UInt(value) => Ok(Value::U64(value)),
        mysql::Value::Float(value) => Ok(Value::F64(value.into())),
        mysql::Value::Double(value) => Ok(Value::F64(value)),
        mysql::Value::Bytes(_value)
            if matches!(
                ty,
                ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL
            ) =>
        {
            Err(DatabaseError::new(
                ErrorKind::Decode,
                "MySQL decimal values require a precision-preserving framework type",
            ))
        }
        mysql::Value::Bytes(_value) if ty == ColumnType::MYSQL_TYPE_JSON => Err(DatabaseError::new(
            ErrorKind::Decode,
            "MySQL JSON values require an explicit framework type",
        )),
        mysql::Value::Bytes(value) if flags.contains(ColumnFlags::BINARY_FLAG) => {
            Ok(Value::Bytes(value))
        }
        mysql::Value::Bytes(value) => String::from_utf8(value)
            .map(Value::Text)
            .map_err(|error| DatabaseError::new(ErrorKind::Decode, error.to_string())),
        mysql::Value::Date(..) | mysql::Value::Time(..) => Err(DatabaseError::new(
            ErrorKind::Decode,
            "MySQL date and time values require an explicit framework type",
        )),
    }
}
