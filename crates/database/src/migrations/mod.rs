mod compiler;
mod operations;
mod plan;
mod schema;

pub use compiler::{
    compile_alter, compile_comments, compile_create, compile_indexes, compile_rebuild,
    compile_table_operation,
};
pub use operations::{AlterOperation, AlterTable, RebuildTable, TableOperation};
pub use plan::{MigrationOperation, MigrationPlan};
pub use schema::{
    Check, Column, ColumnDefault, ColumnType, CreateTable, ForeignAction, ForeignKey, Index, Table,
    Unique,
};

use crate::{
    Capability, Connection, DatabaseError, Direction, Driver, ErrorKind, Query, Result, Row,
    Statement, TransactionOptions, Value,
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

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedMigration {
    pub name: String,
    pub statements: Vec<Statement>,
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

    pub fn plan(&self, connection: &mut dyn Connection) -> Result<Vec<PlannedMigration>> {
        let applied = self.applied(connection)?;
        let driver = connection.driver();
        self.migrations
            .iter()
            .filter(|migration| !applied.iter().any(|item| item.name == migration.name()))
            .map(|migration| {
                let statements = migration.up(driver)?;
                if statements.is_empty() {
                    return Err(error(
                        "a migration direction must contain at least one statement",
                    ));
                }
                Ok(PlannedMigration {
                    name: migration.name().into(),
                    statements,
                })
            })
            .collect()
    }

    pub fn migrate(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        with_migration_lock(connection, |connection| self.migrate_locked(connection))
    }

    fn migrate_locked(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
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
            apply_migration(connection, *migration, batch)?;
            report.applied.push(migration.name().into());
        }
        Ok(report)
    }

    pub fn rollback_last(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        with_migration_lock(connection, |connection| self.rollback_last_locked(connection))
    }

    fn rollback_last_locked(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
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
            revert_migration(connection, *migration)?;
            report.rolled_back.push(migration.name().into());
        }
        Ok(report)
    }

    pub fn rollback_all(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        with_migration_lock(connection, |connection| self.rollback_all_locked(connection))
    }

    fn rollback_all_locked(&self, connection: &mut dyn Connection) -> Result<MigrationReport> {
        let mut report = MigrationReport::default();
        loop {
            let batch = self.rollback_last_locked(connection)?;
            if batch.rolled_back.is_empty() {
                return Ok(report);
            }
            report.rolled_back.extend(batch.rolled_back);
        }
    }
}


fn apply_migration(
    connection: &mut dyn Connection,
    migration: &dyn Migration,
    batch: u64,
) -> Result<()> {
    let driver = connection.driver();
    let steps = migration.up(driver)?;
    if steps.is_empty() {
        return Err(error(
            "a migration direction must contain at least one statement",
        ));
    }
    let tracking = Query::table(TABLE)
        .insert([
            ("name", Value::from(migration.name())),
            ("batch", Value::from(batch)),
        ])
        .to_statement(driver)?;

    if connection
        .capabilities()
        .supports(Capability::TransactionalDdl)
        && !matches!(driver, Driver::Sqlite)
    {
        let mut transaction = connection.begin(TransactionOptions::default())?;
        for statement in steps {
            if let Err(error) = transaction.execute(&statement) {
                let _ = transaction.rollback();
                return Err(error);
            }
        }
        if let Err(error) = transaction.execute(&tracking) {
            let _ = transaction.rollback();
            return Err(error);
        }
        transaction.commit()
    } else {
        execute_steps(connection, steps)?;
        connection.execute(&tracking).map(|_| ())
    }
}

fn revert_migration(connection: &mut dyn Connection, migration: &dyn Migration) -> Result<()> {
    let driver = connection.driver();
    let steps = migration.down(driver)?;
    if steps.is_empty() {
        return Err(error(
            "a migration direction must contain at least one statement",
        ));
    }
    let tracking = Query::table(TABLE)
        .where_("name", "=", migration.name())
        .delete()
        .to_statement(driver)?;

    if connection
        .capabilities()
        .supports(Capability::TransactionalDdl)
        && !matches!(driver, Driver::Sqlite)
    {
        let mut transaction = connection.begin(TransactionOptions::default())?;
        for statement in steps {
            if let Err(error) = transaction.execute(&statement) {
                let _ = transaction.rollback();
                return Err(error);
            }
        }
        if let Err(error) = transaction.execute(&tracking) {
            let _ = transaction.rollback();
            return Err(error);
        }
        transaction.commit()
    } else {
        execute_steps(connection, steps)?;
        connection.execute(&tracking).map(|_| ())
    }
}

fn with_migration_lock<T>(
    connection: &mut dyn Connection,
    operation: impl FnOnce(&mut dyn Connection) -> Result<T>,
) -> Result<T> {
    acquire_migration_lock(connection)?;
    let result = operation(connection);
    let release = release_migration_lock(connection, result.is_ok());
    match (result, release) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn acquire_migration_lock(connection: &mut dyn Connection) -> Result<()> {
    match connection.driver() {
        Driver::Postgres => connection
            .query(&Statement::new("SELECT pg_advisory_lock(1650553701)"))
            .map(|_| ()),
        Driver::MySql => {
            let rows = connection.query(&Statement::new(
                "SELECT GET_LOCK('berserk_migrations', 30) AS acquired",
            ))?;
            let acquired = rows
                .first()
                .and_then(|row| row.get("acquired"))
                .is_some_and(|value| matches!(value, Value::I64(1) | Value::U64(1) | Value::Bool(true)));
            if acquired {
                Ok(())
            } else {
                Err(error("could not acquire the migration lock"))
            }
        }
        Driver::Sqlite => connection
            .execute(&Statement::new("BEGIN IMMEDIATE"))
            .map(|_| ()),
    }
}

fn release_migration_lock(connection: &mut dyn Connection, commit: bool) -> Result<()> {
    match connection.driver() {
        Driver::Postgres => connection
            .query(&Statement::new("SELECT pg_advisory_unlock(1650553701)"))
            .map(|_| ()),
        Driver::MySql => connection
            .query(&Statement::new(
                "SELECT RELEASE_LOCK('berserk_migrations') AS released",
            ))
            .map(|_| ()),
        Driver::Sqlite => connection
            .execute(&Statement::new(if commit { "COMMIT" } else { "ROLLBACK" }))
            .map(|_| ()),
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
