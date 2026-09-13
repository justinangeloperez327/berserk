use crate::{Column, DatabaseError, ErrorKind, Result, Row, Statement, Value};
use postgres::{
    types::{ToSql, Type},
    GenericClient,
};

enum Parameter {
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
}

fn parameters(statement: &Statement) -> Result<Vec<Parameter>> {
    statement.bindings().iter().map(|value| match value {
        Value::Bool(value) => Ok(Parameter::Bool(*value)),
        Value::I64(value) => Ok(Parameter::I64(*value)),
        Value::U64(value) => i64::try_from(*value).map(Parameter::I64).map_err(|_| {
            DatabaseError::new(ErrorKind::Decode, "PostgreSQL has no unsigned 64-bit integer type")
        }),
        Value::F64(value) if value.is_finite() => Ok(Parameter::F64(*value)),
        Value::F64(_) => Err(DatabaseError::new(ErrorKind::Decode, "non-finite floats are not supported")),
        Value::Text(value) => Ok(Parameter::Text(value.clone())),
        Value::Bytes(value) => Ok(Parameter::Bytes(value.clone())),
        Value::Null => Err(DatabaseError::new(
            ErrorKind::Decode,
            "untyped NULL cannot be bound; use a typed database value when typed values are added",
        )),
    }).collect()
}

fn parameter_refs(values: &[Parameter]) -> Vec<&(dyn ToSql + Sync)> {
    values
        .iter()
        .map(|value| match value {
            Parameter::Bool(value) => value as &(dyn ToSql + Sync),
            Parameter::I64(value) => value as &(dyn ToSql + Sync),
            Parameter::F64(value) => value as &(dyn ToSql + Sync),
            Parameter::Text(value) => value as &(dyn ToSql + Sync),
            Parameter::Bytes(value) => value as &(dyn ToSql + Sync),
        })
        .collect()
}

pub(super) fn execute(client: &mut impl GenericClient, statement: &Statement) -> Result<u64> {
    let values = parameters(statement)?;
    let refs = parameter_refs(&values);
    client
        .execute(statement.sql(), &refs)
        .map_err(super::error::map_error)
}

pub(super) fn query(client: &mut impl GenericClient, statement: &Statement) -> Result<Vec<Row>> {
    let values = parameters(statement)?;
    let refs = parameter_refs(&values);
    client
        .query(statement.sql(), &refs)
        .map_err(super::error::map_error)?
        .iter()
        .map(convert_row)
        .collect()
}

fn convert_row(row: &postgres::Row) -> Result<Row> {
    let mut columns = Vec::with_capacity(row.len());
    for (index, metadata) in row.columns().iter().enumerate() {
        let value = convert_value(row, index, metadata.type_())?;
        columns.push(Column::new(metadata.name(), value));
    }
    Row::new(columns)
}

fn convert_value(row: &postgres::Row, index: usize, ty: &Type) -> Result<Value> {
    macro_rules! nullable {
        ($kind:ty, $map:expr) => {{
            let value = row
                .try_get::<_, Option<$kind>>(index)
                .map_err(super::error::map_error)?;
            Ok(value.map($map).unwrap_or(Value::Null))
        }};
    }
    if ty == &Type::BOOL {
        nullable!(bool, Value::Bool)
    } else if ty == &Type::INT2 {
        nullable!(i16, |value| Value::I64(value.into()))
    } else if ty == &Type::INT4 {
        nullable!(i32, |value| Value::I64(value.into()))
    } else if ty == &Type::INT8 {
        nullable!(i64, Value::I64)
    } else if ty == &Type::OID {
        nullable!(u32, |value| Value::U64(value.into()))
    } else if ty == &Type::FLOAT4 {
        nullable!(f32, |value| Value::F64(value.into()))
    } else if ty == &Type::FLOAT8 {
        nullable!(f64, Value::F64)
    } else if matches!(ty, value if value == &Type::TEXT || value == &Type::VARCHAR || value == &Type::BPCHAR || value == &Type::NAME)
    {
        nullable!(String, Value::Text)
    } else if ty == &Type::BYTEA {
        nullable!(Vec<u8>, Value::Bytes)
    } else {
        Err(DatabaseError::new(
            ErrorKind::Decode,
            format!("unsupported PostgreSQL column type: {}", ty.name()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_reject_untyped_null_and_large_unsigned_values() {
        assert!(parameters(&Statement::new("select $1").bind(Value::Null)).is_err());
        assert!(parameters(&Statement::new("select $1").bind(u64::MAX)).is_err());
    }
}
