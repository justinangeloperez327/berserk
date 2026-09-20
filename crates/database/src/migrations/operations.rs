use super::Column;
use crate::{DatabaseError, ErrorKind, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum AlterOperation {
    Add(Column),
    Drop(String),
    Rename { from: String, to: String },
    Modify(Column),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlterTable {
    pub(crate) name: String,
    pub(crate) operations: Vec<AlterOperation>,
}

impl AlterTable {
    pub fn add<const N: usize>(mut self, columns: [Column; N]) -> Self {
        self.operations
            .extend(columns.into_iter().map(AlterOperation::Add));
        self
    }

    pub fn drop<const N: usize>(mut self, columns: [&str; N]) -> Self {
        self.operations.extend(
            columns
                .into_iter()
                .map(|name| AlterOperation::Drop(name.to_owned())),
        );
        self
    }

    pub fn rename(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::Rename {
            from: from.into(),
            to: to.into(),
        });
        self
    }

    pub fn modify(mut self, column: Column) -> Self {
        self.operations.push(AlterOperation::Modify(column));
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_identifier("table", &self.name)?;
        if self.operations.is_empty() {
            return Err(error("an alter table migration must contain at least one operation"));
        }
        for operation in &self.operations {
            match operation {
                AlterOperation::Add(column) | AlterOperation::Modify(column) => {
                    validate_identifier("column", column.name())?;
                }
                AlterOperation::Drop(column) => validate_identifier("column", column)?,
                AlterOperation::Rename { from, to } => {
                    validate_identifier("column", from)?;
                    validate_identifier("column", to)?;
                    if from == to {
                        return Err(error("renamed column must have a different name"));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn operations(&self) -> &[AlterOperation] {
        &self.operations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableOperation {
    Rename { from: String, to: String },
    Drop { name: String, if_exists: bool },
}

impl TableOperation {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Rename { from, to } => {
                validate_identifier("table", from)?;
                validate_identifier("table", to)?;
                if from == to {
                    return Err(error("renamed table must have a different name"));
                }
            }
            Self::Drop { name, .. } => validate_identifier("table", name)?,
        }
        Ok(())
    }
}

impl super::Table {
    pub fn alter(name: impl Into<String>) -> AlterTable {
        AlterTable {
            name: name.into(),
            operations: Vec::new(),
        }
    }

    pub fn rename(from: impl Into<String>, to: impl Into<String>) -> TableOperation {
        TableOperation::Rename {
            from: from.into(),
            to: to.into(),
        }
    }

    pub fn drop(name: impl Into<String>) -> TableOperation {
        TableOperation::Drop {
            name: name.into(),
            if_exists: false,
        }
    }

    pub fn drop_if_exists(name: impl Into<String>) -> TableOperation {
        TableOperation::Drop {
            name: name.into(),
            if_exists: true,
        }
    }
}

fn validate_identifier(kind: &str, identifier: &str) -> Result<()> {
    let valid = !identifier.is_empty()
        && identifier
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        && identifier
            .chars()
            .next()
            .is_some_and(|character| character == '_' || character.is_ascii_alphabetic());
    if valid {
        Ok(())
    } else {
        Err(error(format!("invalid {kind} identifier `{identifier}`")))
    }
}

fn error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Query, message)
}
