mod schema;

pub use schema::{Column, ColumnType, CreateTable, Table};

use crate::{
    Connection, DatabaseError, Direction, Driver, ErrorKind, Query, Result, Row, Statement, Value,
};

const TABLE: &str = "__framework_migrations";

pub trait Migration {
    fn name(&self) -> &'static str;
    fn up(&self, driver: Driver) -> Result<Vec<Statement>>;
    fn down(&self, driver: Driver) -> Result<Vec<Statement>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedMigration {
    pub name: String,
    pub batch: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MigrationReport {
    pub applied: Vec<String>,
    pub rolled_back: Vec<String>,
}

pub struct MigrationRunner<'a> {
    migrations: Vec<&'a dyn Migration>,
}

impl<'a> MigrationRunner<'a> {
    pub fn new(migrations: impl IntoIterator<Item = &'a dyn Migration>) -> Result<Self> {
        let migrations: Vec<_> = migrations.into_iter().collect();
        for (index, migration) in migrations.iter().enumerate() {
            let name = migration.name();
            if name.is_empty() {
                return Err(error("migration names cannot be empty"));
            }
            if migrations[..index].iter().any(|other| other.name() == name) {
                return Err(error(format!("duplicate migration name `{name}`")));
            }
        }
        Ok(Self { migrations })
    }

    pub fn prepare(&self, connection: &mut dyn Connection) -> Result<()> {
        let sql = match connection.driver() {
            Driver::Postgres => "CREATE TABLE IF NOT EXISTS \"__framework_migrations\" (\"name\" VARCHAR(255) PRIMARY KEY, \"batch\" BIGINT NOT NULL)",
            Driver::MySql => "CREATE TABLE IF NOT EXISTS `__framework_migrations` (`name` VARCHAR(255) PRIMARY KEY, `batch` BIGINT UNSIGNED NOT NULL)",
            Driver::Sqlite => "CREATE TABLE IF NOT EXISTS \"__framework_migrations\" (\"name\" TEXT PRIMARY KEY, \"batch\" INTEGER NOT NULL)",
        };
        connection.execute(&Statement::new(sql)).map(|_| ())
    }

    pub fn applied(&self, connection: &mut dyn Connection) -> Result<Vec<AppliedMigration>> {
        self.prepare(connection)?;
        Query::table(TABLE)
            .select(["name", "batch"])
            .order_by("batch", Direction::Asc)
            .order_by("name", Direction::Asc)
            .get(connection)?
            .iter()
            .map(decode_applied)
            .collect()
    }

    pub fn pending(&self, connection: &mut dyn Connection) -> Result<Vec<&'static str>> {
        let applied = self.applied(connection)?;
        Ok(self
            .migrations
            .iter()
            .filter(|migration| !applied.iter().any(|item| item.name == migration.name()))
            .map(|migration| migration.name())
            .collect())
    }

    pub fn migrate(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        let applied = self.applied(connection)?;
        let batch = applied
            .iter()
            .map(|item| item.batch)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| error("migration batch overflow"))?;
        let mut report = MigrationReport::default();
        for migration in &self.migrations {
            if applied.iter().any(|item| item.name == migration.name()) {
                continue;
            }
            let driver = connection.driver();
            let steps = migration.up(driver)?;
            execute_steps(connection, steps)?;
            Query::table(TABLE)
                .insert([
                    ("name", Value::from(migration.name())),
                    ("batch", Value::from(batch)),
                ])
                .execute(connection)?;
            report.applied.push(migration.name().into());
        }
        Ok(report)
    }

    pub fn rollback_last(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        let applied = self.applied(connection)?;
        let Some(batch) = applied.iter().map(|item| item.batch).max() else {
            return Ok(MigrationReport::default());
        };
        let batch_items: Vec<_> = applied.iter().filter(|item| item.batch == batch).collect();
        for item in &batch_items {
            if !self
                .migrations
                .iter()
                .any(|migration| migration.name() == item.name)
            {
                return Err(error(format!(
                    "applied migration `{}` is not registered",
                    item.name
                )));
            }
        }
        let mut report = MigrationReport::default();
        for migration in self
            .migrations
            .iter()
            .rev()
            .filter(|migration| batch_items.iter().any(|item| item.name == migration.name()))
        {
            let driver = connection.driver();
            let steps = migration.down(driver)?;
            execute_steps(connection, steps)?;
            Query::table(TABLE)
                .where_("name", "=", migration.name())
                .delete()
                .execute(connection)?;
            report.rolled_back.push(migration.name().into());
        }
        Ok(report)
    }

    pub fn rollback_all(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        let mut report = MigrationReport::default();
        loop {
            let batch = self.rollback_last(connection)?;
            if batch.rolled_back.is_empty() {
                return Ok(report);
            }
            report.rolled_back.extend(batch.rolled_back);
        }
    }
}

fn decode_applied(row: &Row) -> Result<AppliedMigration> {
    let name = match row.get("name") {
        Some(Value::Text(value)) => value.clone(),
        Some(_) => return Err(decode_error("migration `name` column must be text")),
        None => return Err(decode_error("migration row is missing the `name` column")),
    };
    let batch = match row.get("batch") {
        Some(Value::U64(value)) => *value,
        Some(Value::I64(value)) => u64::try_from(*value)
            .map_err(|_| decode_error("migration `batch` column cannot be negative"))?,
        Some(_) => return Err(decode_error("migration `batch` column must be an integer")),
        None => return Err(decode_error("migration row is missing the `batch` column")),
    };
    Ok(AppliedMigration { name, batch })
}

fn execute_steps(connection: &mut dyn Connection, steps: Vec<Statement>) -> Result<()> {
    if steps.is_empty() {
        return Err(error(
            "a migration direction must contain at least one statement",
        ));
    }
    for statement in steps {
        connection.execute(&statement)?;
    }
    Ok(())
}

fn error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Query, message)
}

fn decode_error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Decode, message)
}
