use crate::{DatabaseError, ErrorKind, Result, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    name: String,
    value: Value,
}

impl Column {
    pub fn new(name: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    columns: Vec<Column>,
}

impl Row {
    pub fn new(columns: Vec<Column>) -> Result<Self> {
        for (index, column) in columns.iter().enumerate() {
            if column.name.is_empty()
                || columns[..index].iter().any(|item| item.name == column.name)
            {
                return Err(DatabaseError::new(
                    ErrorKind::Decode,
                    "row column names must be nonempty and unique",
                ));
            }
        }
        Ok(Self { columns })
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.columns
            .iter()
            .find(|column| column.name == name)
            .map(Column::value)
    }
    pub fn get_at(&self, index: usize) -> Option<&Value> {
        self.columns.get(index).map(Column::value)
    }
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
}
