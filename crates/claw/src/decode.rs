use framework_database::{DatabaseError, ErrorKind, Result, Row, Value};

/// Strictly converts one database value into an application field type.
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self>;
}

/// Reads and converts a named field, preserving missing-column and type errors.
pub fn field<T: FromValue>(row: &Row, name: &str) -> Result<T> {
    let value = row
        .get(name)
        .ok_or_else(|| decode_error(format!("missing column `{name}` while decoding a model")))?;
    T::from_value(value)
        .map_err(|error| decode_error(format!("cannot decode column `{name}`: {error}")))
}

impl FromValue for Value {
    fn from_value(value: &Value) -> Result<Self> {
        Ok(value.clone())
    }
}

impl FromValue for bool {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Bool(value) => Ok(*value),
            Value::I64(0) | Value::U64(0) => Ok(false),
            Value::I64(1) | Value::U64(1) => Ok(true),
            _ => Err(type_error("boolean", value)),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::I64(value) => Ok(*value),
            Value::U64(value) => i64::try_from(*value)
                .map_err(|_| decode_error("unsigned integer does not fit in i64")),
            _ => Err(type_error("i64", value)),
        }
    }
}

macro_rules! signed_integer {
    ($($type:ty),+ $(,)?) => {
        $(impl FromValue for $type {
            fn from_value(value: &Value) -> Result<Self> {
                let value = i64::from_value(value)?;
                <$type>::try_from(value).map_err(|_| {
                    decode_error(concat!("integer does not fit in ", stringify!($type)))
                })
            }
        })+
    };
}

signed_integer!(i8, i16, i32);

impl FromValue for u64 {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::U64(value) => Ok(*value),
            Value::I64(value) => u64::try_from(*value)
                .map_err(|_| decode_error("negative integer does not fit in u64")),
            _ => Err(type_error("u64", value)),
        }
    }
}

macro_rules! unsigned_integer {
    ($($type:ty),+ $(,)?) => {
        $(impl FromValue for $type {
            fn from_value(value: &Value) -> Result<Self> {
                let value = u64::from_value(value)?;
                <$type>::try_from(value).map_err(|_| {
                    decode_error(concat!("integer does not fit in ", stringify!($type)))
                })
            }
        })+
    };
}

unsigned_integer!(u8, u16, u32);

impl FromValue for f64 {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::F64(value) => Ok(*value),
            _ => Err(type_error("f64", value)),
        }
    }
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Text(value) => Ok(value.clone()),
            _ => Err(type_error("text", value)),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Bytes(value) => Ok(value.clone()),
            _ => Err(type_error("bytes", value)),
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            value => T::from_value(value).map(Some),
        }
    }
}

fn type_error(expected: &str, actual: &Value) -> DatabaseError {
    decode_error(format!(
        "expected {expected}, received {}",
        value_kind(actual)
    ))
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::I64(_) => "i64",
        Value::U64(_) => "u64",
        Value::F64(_) => "f64",
        Value::Text(_) => "text",
        Value::Bytes(_) => "bytes",
    }
}

fn decode_error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Decode, message)
}
