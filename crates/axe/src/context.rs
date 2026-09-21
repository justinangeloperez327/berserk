use crate::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Context {
    values: BTreeMap<String, Value>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<Value>) -> Option<Value> {
        self.values.insert(name.into(), value.into())
    }

    pub fn with(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        let _ = self.insert(name, value);
        self
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl From<()> for Context {
    fn from(_: ()) -> Self {
        Self::new()
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Context
where
    K: Into<String>,
    V: Into<Value>,
{
    fn from(values: [(K, V); N]) -> Self {
        values.into_iter().collect()
    }
}

impl<K, V> FromIterator<(K, V)> for Context
where
    K: Into<String>,
    V: Into<Value>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut context = Self::new();
        for (key, value) in iter {
            let _ = context.insert(key, value);
        }
        context
    }
}
