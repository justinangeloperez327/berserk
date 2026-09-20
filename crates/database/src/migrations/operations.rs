use super::{Check, Column, ColumnDefault, CreateTable, ForeignKey, Index, Unique};
use crate::{DatabaseError, ErrorKind, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum AlterOperation {
    Add(Column),
    Drop(String),
    Rename { from: String, to: String },
    Modify(Column),
    AddIndex(Index),
    DropIndex(String),
    AddForeignKey(ForeignKey),
    DropForeignKey(String),
    RenameIndex { from: String, to: String },
    SetDefault { column: String, default: ColumnDefault },
    DropDefault(String),
    AddCheck(Check),
    DropCheck(String),
    AddUnique(Unique),
    DropUnique(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlterTable {
    pub(crate) name: String,
    pub(crate) operations: Vec<AlterOperation>,
}

impl AlterTable {
    pub fn add_columns<const N: usize>(mut self, columns: [Column; N]) -> Self {
        self.operations
            .extend(columns.into_iter().map(AlterOperation::Add));
        self
    }

    pub fn drop_columns<const N: usize>(mut self, columns: [&str; N]) -> Self {
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

    pub fn add_index(mut self, index: Index) -> Self {
        self.operations.push(AlterOperation::AddIndex(index));
        self
    }

    pub fn drop_index(mut self, name: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::DropIndex(name.into()));
        self
    }

    pub fn add_foreign_key(mut self, foreign_key: ForeignKey) -> Self {
        self.operations
            .push(AlterOperation::AddForeignKey(foreign_key));
        self
    }

    pub fn drop_foreign_key(mut self, name: impl Into<String>) -> Self {
        self.operations
            .push(AlterOperation::DropForeignKey(name.into()));
        self
    }

    pub fn rename_index(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::RenameIndex {
            from: from.into(),
            to: to.into(),
        });
        self
    }

    pub fn set_default(
        mut self,
        column: impl Into<String>,
        default: impl Into<crate::Value>,
    ) -> Self {
        self.operations.push(AlterOperation::SetDefault {
            column: column.into(),
            default: ColumnDefault::Value(default.into()),
        });
        self
    }

    pub fn set_default_current_timestamp(mut self, column: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::SetDefault {
            column: column.into(),
            default: ColumnDefault::CurrentTimestamp,
        });
        self
    }

    pub fn drop_default(mut self, column: impl Into<String>) -> Self {
        self.operations
            .push(AlterOperation::DropDefault(column.into()));
        self
    }

    pub fn add_check(mut self, check: Check) -> Self {
        self.operations.push(AlterOperation::AddCheck(check));
        self
    }

    pub fn drop_check(mut self, name: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::DropCheck(name.into()));
        self
    }

    pub fn add_unique(mut self, unique: Unique) -> Self {
        self.operations.push(AlterOperation::AddUnique(unique));
        self
    }

    pub fn drop_unique(mut self, name: impl Into<String>) -> Self {
        self.operations.push(AlterOperation::DropUnique(name.into()));
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_identifier("table", &self.name)?;
        if self.operations.is_empty() {
            return Err(error(
                "an alter table migration must contain at least one operation",
            ));
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
                AlterOperation::AddIndex(index) => {
                    if index.columns.is_empty() {
                        return Err(error("an index must contain at least one column"));
                    }
                    for column in &index.columns {
                        validate_identifier("index column", column)?;
                    }
                    if let Some(name) = &index.name {
                        validate_identifier("index", name)?;
                    }
                }
                AlterOperation::DropIndex(name) => validate_identifier("index", name)?,
                AlterOperation::AddForeignKey(foreign_key) => {
                    if foreign_key.columns.is_empty()
                        || foreign_key.columns.len() != foreign_key.referenced_columns.len()
                    {
                        return Err(error("foreign key columns must match referenced columns"));
                    }
                    validate_identifier("referenced table", &foreign_key.referenced_table)?;
                    for column in &foreign_key.columns {
                        validate_identifier("foreign key column", column)?;
                    }
                    for column in &foreign_key.referenced_columns {
                        validate_identifier("referenced column", column)?;
                    }
                    if let Some(name) = &foreign_key.name {
                        validate_identifier("foreign key", name)?;
                    }
                }
                AlterOperation::DropForeignKey(name) => validate_identifier("foreign key", name)?,
                AlterOperation::RenameIndex { from, to } => {
                    validate_identifier("index", from)?;
                    validate_identifier("index", to)?;
                    if from == to {
                        return Err(error("renamed index must have a different name"));
                    }
                }
                AlterOperation::SetDefault { column, .. }
                | AlterOperation::DropDefault(column) => {
                    validate_identifier("column", column)?;
                }
                AlterOperation::AddCheck(check) => {
                    validate_identifier("check constraint", &check.name)?;
                    if check.expression.trim().is_empty() {
                        return Err(error("check constraint expressions cannot be empty"));
                    }
                }
                AlterOperation::DropCheck(name) => validate_identifier("check constraint", name)?,
                AlterOperation::AddUnique(unique) => {
                    if unique.columns.is_empty() {
                        return Err(error("a unique constraint must contain at least one column"));
                    }
                    for column in &unique.columns {
                        validate_identifier("unique constraint column", column)?;
                    }
                    if let Some(name) = &unique.name {
                        validate_identifier("unique constraint", name)?;
                    }
                }
                AlterOperation::DropUnique(name) => {
                    validate_identifier("unique constraint", name)?;
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

#[derive(Clone, Debug, PartialEq)]
pub struct RebuildTable {
    pub(crate) name: String,
    pub(crate) replacement: CreateTable,
    pub(crate) copy: Vec<(String, String)>,
}

impl RebuildTable {
    pub fn copy<const N: usize>(mut self, columns: [(&str, &str); N]) -> Self {
        self.copy.extend(
            columns
                .into_iter()
                .map(|(from, to)| (from.to_owned(), to.to_owned())),
        );
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_identifier("table", &self.name)?;
        self.replacement.validate()?;
        if self.replacement.name() != self.name {
            return Err(error(
                "SQLite rebuild replacement must use the original table name",
            ));
        }
        if self.copy.is_empty() {
            return Err(error("SQLite rebuild requires explicit column copy mappings"));
        }
        for (from, to) in &self.copy {
            validate_identifier("source column", from)?;
            validate_identifier("replacement column", to)?;
            if !self
                .replacement
                .column_definitions()
                .iter()
                .any(|column| column.name() == to)
            {
                return Err(error(format!(
                    "rebuild references unknown replacement column `{to}`"
                )));
            }
        }
        Ok(())
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
    pub fn rebuild(name: impl Into<String>, replacement: CreateTable) -> RebuildTable {
        RebuildTable {
            name: name.into(),
            replacement,
            copy: Vec::new(),
        }
    }

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
        && identifier.len() <= 63
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
