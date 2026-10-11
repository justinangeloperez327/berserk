use crate::{Column, DatabaseError, ErrorKind, Result, Row, Statement, Value};
use bytes::BytesMut;
use postgres::{
    types::{IsNull, ToSql, Type},
    GenericClient,
};

enum Parameter {
    Null,
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    U32(u32),
    F32(f32),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
}

impl std::fmt::Debug for Parameter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Parameter([redacted])")
    }
}

impl ToSql for Parameter {
    fn to_sql(
        &self,
        ty: &Type,
        output: &mut BytesMut,
    ) -> std::result::Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        // Each non-NULL encoder still checks its actual PostgreSQL type.
        match self {
            Self::Null => Ok(IsNull::Yes),
            Self::Bool(value) => value.to_sql_checked(ty, output),
            Self::I16(value) => value.to_sql_checked(ty, output),
            Self::I32(value) => value.to_sql_checked(ty, output),
            Self::I64(value) => value.to_sql_checked(ty, output),
            Self::U32(value) => value.to_sql_checked(ty, output),
            Self::F32(value) => value.to_sql_checked(ty, output),
            Self::F64(value) => value.to_sql_checked(ty, output),
            Self::Text(value) => value.to_sql_checked(ty, output),
            Self::Bytes(value) => value.to_sql_checked(ty, output),
        }
    }

    fn accepts(_: &Type) -> bool {
        // NULL works with every server-inferred type. Other variants perform
        // the checked delegation above, including rejection of mismatches.
        true
    }

    postgres::types::to_sql_checked!();
}

fn integer(value: i64, ty: &Type) -> Result<Parameter> {
    let overflow = || {
        DatabaseError::new(
            ErrorKind::Decode,
            "integer exceeds PostgreSQL parameter range",
        )
    };
    if ty == &Type::INT2 {
        i16::try_from(value)
            .map(Parameter::I16)
            .map_err(|_| overflow())
    } else if ty == &Type::INT4 {
        i32::try_from(value)
            .map(Parameter::I32)
            .map_err(|_| overflow())
    } else if ty == &Type::OID {
        u32::try_from(value)
            .map(Parameter::U32)
            .map_err(|_| overflow())
    } else {
        Ok(Parameter::I64(value))
    }
}

fn parameters(statement: &Statement, types: &[Type]) -> Result<Vec<Parameter>> {
    if statement.bindings().len() != types.len() {
        return Err(DatabaseError::new(
            ErrorKind::InvalidInput,
            "PostgreSQL parameter count does not match bindings",
        ));
    }
    statement
        .bindings()
        .iter()
        .zip(types)
        .map(|(value, ty)| match value {
            Value::Null => Ok(Parameter::Null),
            Value::Bool(value) => Ok(Parameter::Bool(*value)),
            Value::I64(value) => integer(*value, ty),
            Value::U64(value) => {
                let value = i64::try_from(*value).map_err(|_| {
                    DatabaseError::new(
                        ErrorKind::Decode,
                        "PostgreSQL has no unsigned 64-bit integer type",
                    )
                })?;
                integer(value, ty)
            }
            Value::F64(value) if value.is_finite() => {
                if ty == &Type::FLOAT4 {
                    let narrowed = *value as f32;
                    if !narrowed.is_finite() {
                        return Err(DatabaseError::new(
                            ErrorKind::Decode,
                            "float exceeds PostgreSQL REAL range",
                        ));
                    }
                    Ok(Parameter::F32(narrowed))
                } else {
                    Ok(Parameter::F64(*value))
                }
            }
            Value::F64(_) => Err(DatabaseError::new(
                ErrorKind::Decode,
                "non-finite floats are not supported",
            )),
            Value::Text(value) => Ok(Parameter::Text(value.clone())),
            Value::Bytes(value) => Ok(Parameter::Bytes(value.clone())),
        })
        .collect()
}

fn parameter_refs(values: &[Parameter]) -> Vec<&(dyn ToSql + Sync)> {
    values
        .iter()
        .map(|value| value as &(dyn ToSql + Sync))
        .collect()
}

pub(super) fn execute(client: &mut impl GenericClient, statement: &Statement) -> Result<u64> {
    let prepared = client
        .prepare(statement.sql())
        .map_err(super::error::map_error)?;
    let values = parameters(statement, prepared.params())?;
    let refs = parameter_refs(&values);
    client
        .execute(&prepared, &refs)
        .map_err(super::error::map_error)
}

pub(super) fn query(client: &mut impl GenericClient, statement: &Statement) -> Result<Vec<Row>> {
    let prepared = client
        .prepare(statement.sql())
        .map_err(super::error::map_error)?;
    let values = parameters(statement, prepared.params())?;
    let refs = parameter_refs(&values);
    client
        .query(&prepared, &refs)
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
    fn null_uses_the_server_inferred_type_without_serializing_bytes() {
        for ty in [Type::TEXT, Type::INT4, Type::BOOL, Type::BYTEA] {
            let values = parameters(
                &Statement::new("select $1").bind(Value::Null),
                std::slice::from_ref(&ty),
            )
            .unwrap();
            let mut output = BytesMut::new();
            assert!(matches!(
                values[0].to_sql_checked(&ty, &mut output).unwrap(),
                IsNull::Yes
            ));
            assert!(output.is_empty());
        }
    }

    #[test]
    fn numeric_parameters_match_server_types_and_reject_overflow() {
        for ty in [Type::INT2, Type::INT4, Type::INT8, Type::OID] {
            let values = parameters(
                &Statement::new("select $1").bind(42_i32),
                std::slice::from_ref(&ty),
            )
            .unwrap();
            assert!(values[0].to_sql_checked(&ty, &mut BytesMut::new()).is_ok());
        }
        for ty in [Type::FLOAT4, Type::FLOAT8] {
            let values = parameters(
                &Statement::new("select $1").bind(1.25_f64),
                std::slice::from_ref(&ty),
            )
            .unwrap();
            assert!(values[0].to_sql_checked(&ty, &mut BytesMut::new()).is_ok());
        }
        assert!(parameters(&Statement::new("select $1").bind(32_768_i64), &[Type::INT2]).is_err());
        assert!(parameters(&Statement::new("select $1").bind(i64::MAX), &[Type::INT4]).is_err());
        assert!(parameters(&Statement::new("select $1").bind(u64::MAX), &[Type::INT8]).is_err());
        assert!(parameters(&Statement::new("select $1").bind(f64::MAX), &[Type::FLOAT4]).is_err());
        assert!(parameters(&Statement::new("select $1").bind(f64::NAN), &[Type::FLOAT8]).is_err());
    }

    #[test]
    fn incompatible_non_null_values_remain_errors() {
        let values =
            parameters(&Statement::new("select $1").bind("secret"), &[Type::INT4]).unwrap();
        assert!(values[0]
            .to_sql_checked(&Type::INT4, &mut BytesMut::new())
            .is_err());
        assert!(!format!("{:?}", values[0]).contains("secret"));
    }
}
