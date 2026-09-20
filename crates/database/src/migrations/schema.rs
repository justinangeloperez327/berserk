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
    Boolean,
    Date,
    Time,
    DateTime,
    Timestamp,
    Json,
    Binary,
    Uuid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub(crate) name: String,
    pub(crate) kind: ColumnType,
    pub(crate) nullable: bool,
    pub(crate) unique: bool,
    pub(crate) primary: bool,
    pub(crate) default: Option<Value>,
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
        self.default = Some(value.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &ColumnType {
        &self.kind
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTable {
    pub(crate) name: String,
    pub(crate) columns: Vec<Column>,
}

impl CreateTable {
    pub fn columns<const N: usize>(mut self, columns: [Column; N]) -> Self {
        self.columns.extend(columns);
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
