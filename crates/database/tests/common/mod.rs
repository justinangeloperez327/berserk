use berserk_database::{Connection, Driver, Migration, MigrationRunner, Result, Statement};

const FIRST_TABLE: &str = "berserk_migration_first";
const SECOND_TABLE: &str = "berserk_migration_second";
const FAILED_TABLE: &str = "berserk_migration_failed";

struct CreateFirst;

impl Migration for CreateFirst {
    fn name(&self) -> &'static str {
        "202609170001_create_first"
    }

    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(format!(
            "CREATE TABLE {FIRST_TABLE} (id BIGINT PRIMARY KEY)"
        ))])
    }

    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(format!("DROP TABLE {FIRST_TABLE}"))])
    }
}

struct CreateSecond;

impl Migration for CreateSecond {
    fn name(&self) -> &'static str {
        "202609170002_create_second"
    }

    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(format!(
            "CREATE TABLE {SECOND_TABLE} (id BIGINT PRIMARY KEY)"
        ))])
    }

    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(format!("DROP TABLE {SECOND_TABLE}"))])
    }
}

struct FailingMigration;

impl Migration for FailingMigration {
    fn name(&self) -> &'static str {
        "202609170003_failing_migration"
    }

    fn up(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![
            Statement::new(format!(
                "CREATE TABLE {FAILED_TABLE} (id BIGINT PRIMARY KEY)"
            )),
            Statement::new("THIS IS NOT VALID SQL"),
        ])
    }

    fn down(&self, _driver: Driver) -> Result<Vec<Statement>> {
        Ok(vec![Statement::new(format!("DROP TABLE {FAILED_TABLE}"))])
    }
}

pub fn run_live_migration_contract(connection: &mut dyn Connection) {
    clean(connection);

    let first = CreateFirst;
    let second = CreateSecond;
    let runner = MigrationRunner::new([&first as &dyn Migration, &second]).unwrap();

    assert_eq!(
        runner.pending(connection).unwrap(),
        vec![first.name(), second.name()]
    );
    let report = runner.migrate(connection).unwrap();
    assert_eq!(report.applied, vec![first.name(), second.name()]);
    assert!(runner.pending(connection).unwrap().is_empty());
    assert!(runner.migrate(connection).unwrap().applied.is_empty());
    assert!(connection
        .query(&Statement::new(format!("SELECT id FROM {FIRST_TABLE}")))
        .is_ok());
    assert!(connection
        .query(&Statement::new(format!("SELECT id FROM {SECOND_TABLE}")))
        .is_ok());

    let report = runner.rollback_last(connection).unwrap();
    assert_eq!(report.rolled_back, vec![second.name(), first.name()]);
    assert!(runner.applied(connection).unwrap().is_empty());
    assert!(connection
        .query(&Statement::new(format!("SELECT id FROM {FIRST_TABLE}")))
        .is_err());
    assert!(connection
        .query(&Statement::new(format!("SELECT id FROM {SECOND_TABLE}")))
        .is_err());

    let failing = FailingMigration;
    let runner = MigrationRunner::new([&failing as &dyn Migration]).unwrap();
    assert!(runner.migrate(connection).is_err());
    assert!(runner.applied(connection).unwrap().is_empty());

    clean(connection);
}

fn clean(connection: &mut dyn Connection) {
    for table in [
        FAILED_TABLE,
        SECOND_TABLE,
        FIRST_TABLE,
        "__framework_migrations",
    ] {
        connection
            .execute(&Statement::new(format!("DROP TABLE IF EXISTS {table}")))
            .unwrap();
    }
}
