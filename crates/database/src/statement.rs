use crate::Value;

#[derive(Clone, PartialEq)]
pub struct Statement {
    sql: String,
    bindings: Vec<Value>,
}

impl std::fmt::Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Statement")
            .field("sql", &"[redacted]")
            .field("sql_bytes", &self.sql.len())
            .field("binding_count", &self.bindings.len())
            .finish()
    }
}

impl Statement {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            bindings: Vec::new(),
        }
    }
    pub fn bind(mut self, value: impl Into<Value>) -> Self {
        self.bindings.push(value.into());
        self
    }
    pub fn sql(&self) -> &str {
        &self.sql
    }
    pub fn bindings(&self) -> &[Value] {
        &self.bindings
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Execution {
    pub affected_rows: u64,
    pub last_insert_id: Option<u64>,
}
