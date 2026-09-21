#[derive(Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum ValueRef<'a> {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(&'a str),
    Bytes(&'a [u8]),
}

impl std::fmt::Debug for ValueRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => f.write_str("ValueRef::Null"),
            Self::Bool(_) => f.write_str("ValueRef::Bool([redacted])"),
            Self::I64(_) => f.write_str("ValueRef::I64([redacted])"),
            Self::U64(_) => f.write_str("ValueRef::U64([redacted])"),
            Self::F64(_) => f.write_str("ValueRef::F64([redacted])"),
            Self::Text(value) => write!(f, "ValueRef::Text([redacted; {} bytes])", value.len()),
            Self::Bytes(value) => write!(f, "ValueRef::Bytes([redacted; {} bytes])", value.len()),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => f.write_str("Value::Null"),
            Self::Bool(_) => f.write_str("Value::Bool([redacted])"),
            Self::I64(_) => f.write_str("Value::I64([redacted])"),
            Self::U64(_) => f.write_str("Value::U64([redacted])"),
            Self::F64(_) => f.write_str("Value::F64([redacted])"),
            Self::Text(value) => write!(f, "Value::Text([redacted; {} bytes])", value.len()),
            Self::Bytes(value) => write!(f, "Value::Bytes([redacted; {} bytes])", value.len()),
        }
    }
}

impl Value {
    pub fn as_ref(&self) -> ValueRef<'_> {
        match self {
            Self::Null => ValueRef::Null,
            Self::Bool(value) => ValueRef::Bool(*value),
            Self::I64(value) => ValueRef::I64(*value),
            Self::U64(value) => ValueRef::U64(*value),
            Self::F64(value) => ValueRef::F64(*value),
            Self::Text(value) => ValueRef::Text(value),
            Self::Bytes(value) => ValueRef::Bytes(value),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Self::I64(value.into())
    }
}
impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Self::I64(value.into())
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::I64(value.into())
    }
}
impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::U64(value.into())
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Self::U64(value.into())
    }
}
impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::U64(value.into())
    }
}
impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}
impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}
