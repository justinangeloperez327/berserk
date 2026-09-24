use berserk_database::{
    Capability, Connection, Driver, Migration, MigrationRunner, Result, Statement,
};

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
    if connection
        .capabilities()
        .supports(Capability::TransactionalDdl)
    {
        assert!(connection
            .query(&Statement::new(format!("SELECT id FROM {FAILED_TABLE}")))
            .is_err());
    }

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


const CONTRACT_PARENT: &str = "berserk_contract_parent";
const CONTRACT_CHILD: &str = "berserk_contract_child";

/// Backend-neutral CRUD, value, constraint, ordering, pagination, and rollback contract.
pub fn run_live_database_contract(connection: &mut dyn Connection) {
    clean_database_contract(connection);
    let driver = connection.driver();

    connection.execute(&Statement::new(format!(
        "CREATE TABLE {CONTRACT_PARENT} (id BIGINT PRIMARY KEY, name VARCHAR(255) NOT NULL UNIQUE, optional_text VARCHAR(255) NULL)"
    ))).unwrap();
    connection.execute(&Statement::new(format!(
        "CREATE TABLE {CONTRACT_CHILD} (id BIGINT PRIMARY KEY, parent_id BIGINT NOT NULL, label VARCHAR(255) NOT NULL, CONSTRAINT fk_berserk_contract_parent FOREIGN KEY (parent_id) REFERENCES {CONTRACT_PARENT}(id))"
    ))).unwrap();

    crate::contract_insert(connection, driver, 1, "Ada", None);
    crate::contract_insert(connection, driver, 2, "Grace λ", Some("unicode ✓"));
    crate::contract_insert(connection, driver, 3, "Linus", Some("third"));

    let rows = crate::contract_select(connection, driver);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("id"), Some(&berserk_database::Value::I64(2)));
    assert_eq!(rows[0].get("name"), Some(&berserk_database::Value::Text("Grace λ".into())));
    assert_eq!(rows[0].get("optional_text"), Some(&berserk_database::Value::Text("unicode ✓".into())));
    assert_eq!(rows[1].get("id"), Some(&berserk_database::Value::I64(1)));
    assert_eq!(rows[1].get("optional_text"), Some(&berserk_database::Value::Null));

    let duplicate = crate::contract_insert_error(connection, driver, 4, "Ada", None);
    assert_eq!(duplicate.kind(), &berserk_database::ErrorKind::UniqueViolation);

    let null_error = crate::contract_null_error(connection, driver, 5);
    assert_eq!(null_error.kind(), &berserk_database::ErrorKind::NotNullViolation);

    let fk_error = crate::contract_fk_error(connection, driver);
    assert_eq!(fk_error.kind(), &berserk_database::ErrorKind::ForeignKeyViolation);

    {
        let mut tx = connection.begin(Default::default()).unwrap();
        crate::contract_insert(&mut tx, driver, 10, "Rolled Back", None);
        tx.rollback().unwrap();
    }
    assert!(!crate::contract_exists(connection, driver, 10));

    clean_database_contract(connection);
}

fn clean_database_contract(connection: &mut dyn Connection) {
    for table in [CONTRACT_CHILD, CONTRACT_PARENT] {
        connection.execute(&Statement::new(format!("DROP TABLE IF EXISTS {table}"))).unwrap();
    }
}
