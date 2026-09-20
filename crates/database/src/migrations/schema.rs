use crate::{DatabaseError, ErrorKind, Result, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColumnType {
    Id,
    String(Option<u32>),
    Text,
    TinyInteger,
    SmallInteger,
    Integer,
    BigInteger,
    Decimal { precision: u8, scale: u8 },
    Float,
    Double,
    Boolean,
    Date,
    Time,
    DateTime,
    Timestamp,
    Json,
    Binary,
    Uuid,
    Enum(Vec<String>),
    Custom(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnDefault {
    Value(Value),
    CurrentTimestamp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub(crate) name: String,
    pub(crate) kind: ColumnType,
    pub(crate) nullable: bool,
    pub(crate) unique: bool,
    pub(crate) primary: bool,
    pub(crate) default: Option<ColumnDefault>,
    pub(crate) generated: Option<String>,
    pub(crate) comment: Option<String>,
}

impl Column {
    fn new(name: impl Into<String>, kind: ColumnType) -> Self {
        Self {
            name: name.into(),
            kind,
            nullable: false,
            unique: false,
            primary: false,
            default: None,
            generated: None,
            comment: None,
        }
    }

    pub fn id() -> Self {
        Self {
            primary: true,
            ..Self::new("id", ColumnType::Id)
        }
    }

    pub fn string(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::String(None))
    }

    pub fn text(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Text)
    }

    pub fn tiny_integer(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::TinyInteger)
    }

    pub fn small_integer(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::SmallInteger)
    }

    pub fn integer(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Integer)
    }

    pub fn big_integer(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::BigInteger)
    }

    pub fn decimal(name: impl Into<String>, precision: u8, scale: u8) -> Self {
        Self::new(name, ColumnType::Decimal { precision, scale })
    }

    pub fn float(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Float)
    }

    pub fn double(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Double)
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Boolean)
    }

    pub fn date(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Date)
    }

    pub fn time(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Time)
    }

    pub fn datetime(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::DateTime)
    }

    pub fn timestamp(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Timestamp)
    }

    pub fn json(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Json)
    }

    pub fn binary(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Binary)
    }

    pub fn uuid(name: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Uuid)
    }

    pub fn enum_<I, S>(name: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            name,
            ColumnType::Enum(values.into_iter().map(Into::into).collect()),
        )
    }

    pub fn custom(name: impl Into<String>, sql_type: impl Into<String>) -> Self {
        Self::new(name, ColumnType::Custom(sql_type.into()))
    }

    pub fn length(mut self, length: u32) -> Self {
        if matches!(self.kind, ColumnType::String(_)) {
            self.kind = ColumnType::String(Some(length));
        }
        self
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn default(mut self, value: impl Into<Value>) -> Self {
        self.default = Some(ColumnDefault::Value(value.into()));
        self
    }

    pub fn default_current_timestamp(mut self) -> Self {
        self.default = Some(ColumnDefault::CurrentTimestamp);
        self
    }

    pub fn generated_stored(mut self, expression: impl Into<String>) -> Self {
        self.generated = Some(expression.into());
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &ColumnType {
        &self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForeignAction {
    Cascade,
    Restrict,
    SetNull,
    NoAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignKey {
    pub(crate) columns: Vec<String>,
    pub(crate) referenced_table: String,
    pub(crate) referenced_columns: Vec<String>,
    pub(crate) on_delete: Option<ForeignAction>,
    pub(crate) on_update: Option<ForeignAction>,
    pub(crate) name: Option<String>,
}

impl ForeignKey {
    pub fn new<const N: usize>(columns: [&str; N]) -> Self {
        Self {
            columns: columns.into_iter().map(str::to_owned).collect(),
            referenced_table: String::new(),
            referenced_columns: Vec::new(),
            on_delete: None,
            on_update: None,
            name: None,
        }
    }

    pub fn references<const N: usize>(
        mut self,
        table: impl Into<String>,
        columns: [&str; N],
    ) -> Self {
        self.referenced_table = table.into();
        self.referenced_columns = columns.into_iter().map(str::to_owned).collect();
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn on_delete(mut self, action: ForeignAction) -> Self {
        self.on_delete = Some(action);
        self
    }

    pub fn on_update(mut self, action: ForeignAction) -> Self {
        self.on_update = Some(action);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    pub(crate) name: String,
    pub(crate) expression: String,
}

impl Check {
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            expression: expression.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unique {
    pub(crate) columns: Vec<String>,
    pub(crate) name: Option<String>,
}

impl Unique {
    pub fn new<const N: usize>(columns: [&str; N]) -> Self {
        Self {
            columns: columns.into_iter().map(str::to_owned).collect(),
            name: None,
        }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index {
    pub(crate) columns: Vec<String>,
    pub(crate) unique: bool,
    pub(crate) name: Option<String>,
}

impl Index {
    pub fn new<const N: usize>(columns: [&str; N]) -> Self {
        Self {
            columns: columns.into_iter().map(str::to_owned).collect(),
            unique: false,
            name: None,
        }
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTable {
    pub(crate) name: String,
    pub(crate) columns: Vec<Column>,
    pub(crate) indexes: Vec<Index>,
    pub(crate) foreign_keys: Vec<ForeignKey>,
    pub(crate) primary_key: Option<Vec<String>>,
    pub(crate) checks: Vec<Check>,
    pub(crate) uniques: Vec<Unique>,
    pub(crate) comment: Option<String>,
}

impl CreateTable {
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    pub fn columns<const N: usize>(mut self, columns: [Column; N]) -> Self {
        self.columns.extend(columns);
        self
    }

    pub fn indexes<const N: usize>(mut self, indexes: [Index; N]) -> Self {
        self.indexes.extend(indexes);
        self
    }

    pub fn foreign_keys<const N: usize>(mut self, foreign_keys: [ForeignKey; N]) -> Self {
        self.foreign_keys.extend(foreign_keys);
        self
    }

    pub fn primary<const N: usize>(mut self, columns: [&str; N]) -> Self {
        self.primary_key = Some(columns.into_iter().map(str::to_owned).collect());
        self
    }

    pub fn checks<const N: usize>(mut self, checks: [Check; N]) -> Self {
        self.checks.extend(checks);
        self
    }

    pub fn uniques<const N: usize>(mut self, uniques: [Unique; N]) -> Self {
        self.uniques.extend(uniques);
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_identifier("table", &self.name)?;
        if self.columns.is_empty() {
            return Err(error("a table must contain at least one column"));
        }
        for (index, column) in self.columns.iter().enumerate() {
            validate_identifier("column", &column.name)?;
            if self.columns[..index]
                .iter()
                .any(|other| other.name == column.name)
            {
                return Err(error(format!("duplicate column `{}`", column.name)));
            }
            if let ColumnType::String(Some(0)) = column.kind {
                return Err(error(format!(
                    "string column `{}` must have a positive length",
                    column.name
                )));
            }
            if let ColumnType::Decimal { precision, scale } = column.kind {
                if precision == 0 || scale > precision {
                    return Err(error(format!(
                        "decimal column `{}` has invalid precision or scale",
                        column.name
                    )));
                }
            }
            if let ColumnType::Enum(values) = &column.kind {
                if values.is_empty() || values.iter().any(|value| value.is_empty()) {
                    return Err(error(format!(
                        "enum column `{}` must contain non-empty values",
                        column.name
                    )));
                }
            }
            if let ColumnType::Custom(sql_type) = &column.kind {
                if sql_type.trim().is_empty() {
                    return Err(error(format!(
                        "custom column `{}` must declare a SQL type",
                        column.name
                    )));
                }
            }
            if column
                .generated
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(error(format!(
                    "generated column `{}` must contain an expression",
                    column.name
                )));
            }
            if column.generated.is_some() && column.default.is_some() {
                return Err(error(format!(
                    "generated column `{}` cannot also declare a default",
                    column.name
                )));
            }
        }
        if let Some(primary_key) = &self.primary_key {
            if primary_key.is_empty() {
                return Err(error(
                    "a composite primary key must contain at least one column",
                ));
            }
            if self.columns.iter().any(|column| column.primary) {
                return Err(error("column and table primary keys cannot be combined"));
            }
            for (position, column) in primary_key.iter().enumerate() {
                validate_identifier("primary key column", column)?;
                if primary_key[..position].contains(column) {
                    return Err(error(format!("duplicate primary key column `{column}`")));
                }
                if !self.columns.iter().any(|item| item.name == *column) {
                    return Err(error(format!(
                        "primary key references unknown column `{column}`"
                    )));
                }
            }
        }
        for check in &self.checks {
            validate_identifier("check constraint", &check.name)?;
            if check.expression.trim().is_empty() {
                return Err(error("check constraint expressions cannot be empty"));
            }
        }
        for unique in &self.uniques {
            if unique.columns.is_empty() {
                return Err(error(
                    "a unique constraint must contain at least one column",
                ));
            }
            if let Some(name) = &unique.name {
                validate_identifier("unique constraint", name)?;
            }
            for column in &unique.columns {
                validate_identifier("unique constraint column", column)?;
                if !self.columns.iter().any(|item| item.name == *column) {
                    return Err(error(format!(
                        "unique constraint references unknown column `{column}`"
                    )));
                }
            }
        }
        for (index_position, index) in self.indexes.iter().enumerate() {
            if index.columns.is_empty() {
                return Err(error("an index must contain at least one column"));
            }
            if let Some(name) = &index.name {
                validate_identifier("index", name)?;
            }
            let generated_name = format!("idx_{}_{}", self.name, index.columns.join("_"));
            let index_name = index.name.as_deref().unwrap_or(&generated_name);
            validate_identifier("index", index_name)?;
            if self.indexes[..index_position].iter().any(|other| {
                let other_generated = format!("idx_{}_{}", self.name, other.columns.join("_"));
                other.name.as_deref().unwrap_or(&other_generated) == index_name
            }) {
                return Err(error(format!("duplicate index name `{index_name}`")));
            }
            for (column_position, column) in index.columns.iter().enumerate() {
                validate_identifier("index column", column)?;
                if index.columns[..column_position].contains(column) {
                    return Err(error(format!("duplicate index column `{column}`")));
                }
                if !self.columns.iter().any(|item| item.name == *column) {
                    return Err(error(format!("index references unknown column `{column}`")));
                }
            }
        }
        for foreign_key in &self.foreign_keys {
            if foreign_key.columns.is_empty()
                || foreign_key.columns.len() != foreign_key.referenced_columns.len()
            {
                return Err(error("foreign key columns must match referenced columns"));
            }
            validate_identifier("referenced table", &foreign_key.referenced_table)?;
            if let Some(name) = &foreign_key.name {
                validate_identifier("foreign key", name)?;
            }
            for (position, column) in foreign_key.columns.iter().enumerate() {
                validate_identifier("foreign key column", column)?;
                if foreign_key.columns[..position].contains(column) {
                    return Err(error(format!("duplicate foreign key column `{column}`")));
                }
                if !self.columns.iter().any(|item| item.name == *column) {
                    return Err(error(format!(
                        "foreign key references unknown local column `{column}`"
                    )));
                }
            }
            for (position, column) in foreign_key.referenced_columns.iter().enumerate() {
                validate_identifier("referenced column", column)?;
                if foreign_key.referenced_columns[..position].contains(column) {
                    return Err(error(format!("duplicate referenced column `{column}`")));
                }
            }
            if matches!(foreign_key.on_delete, Some(ForeignAction::SetNull))
                && foreign_key.columns.iter().any(|name| {
                    self.columns
                        .iter()
                        .find(|column| column.name == *name)
                        .is_some_and(|column| !column.nullable)
                })
            {
                return Err(error(
                    "SET NULL foreign keys require nullable local columns",
                ));
            }
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn column_definitions(&self) -> &[Column] {
        &self.columns
    }
}

pub struct Table;

impl Table {
    pub fn create(name: impl Into<String>) -> CreateTable {
        CreateTable {
            name: name.into(),
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            primary_key: None,
            checks: Vec::new(),
            uniques: Vec::new(),
            comment: None,
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
