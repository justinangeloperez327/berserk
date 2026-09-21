use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeHtml(String);

impl SafeHtml {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    Bytes(Vec<u8>),
    SafeHtml(SafeHtml),
    List(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn object<K, I>(values: I) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, Value)>,
    {
        Self::Object(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(value) => value != "0",
            Self::Text(value) => !value.is_empty(),
            Self::Bytes(value) => !value.is_empty(),
            Self::SafeHtml(value) => !value.as_str().is_empty(),
            Self::List(value) => !value.is_empty(),
            Self::Object(value) => !value.is_empty(),
        }
    }

    pub(crate) fn field(&self, name: &str) -> Option<&Self> {
        match self {
            Self::Object(values) => values.get(name),
            _ => None,
        }
    }
}

impl From<SafeHtml> for Value {
    fn from(value: SafeHtml) -> Self {
        Self::SafeHtml(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(values: Vec<T>) -> Self {
        Self::List(values.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<String>> From<BTreeMap<K, Value>> for Value {
    fn from(values: BTreeMap<K, Value>) -> Self {
        Self::Object(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }
}

macro_rules! integer_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Self::Number(value.to_string())
                }
            }
        )*
    };
}

integer_value!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
