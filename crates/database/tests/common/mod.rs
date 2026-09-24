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

    contract_insert(connection, driver, 1, "Ada", None);
    contract_insert(connection, driver, 2, "Grace λ", Some("unicode ✓"));
    contract_insert(connection, driver, 3, "Linus", Some("third"));

    let rows = contract_select(connection, driver);
    assert_eq!(rows.len(), 2);
    assert!(integer_eq(rows[0].get("id"), 2));
    assert_eq!(
        rows[0].get("name"),
        Some(&berserk_database::Value::Text("Grace λ".into()))
    );
    assert_eq!(
        rows[0].get("optional_text"),
        Some(&berserk_database::Value::Text("unicode ✓".into()))
    );
    assert!(integer_eq(rows[1].get("id"), 1));
    assert_eq!(
        rows[1].get("optional_text"),
        Some(&berserk_database::Value::Null)
    );

    let duplicate = contract_insert_error(connection, driver, 4, "Ada", None);
    assert_eq!(
        duplicate.kind(),
        &berserk_database::ErrorKind::UniqueViolation
    );

    let null_error = contract_null_error(connection, driver, 5);
    assert_eq!(
        null_error.kind(),
        &berserk_database::ErrorKind::NotNullViolation
    );

    let fk_error = contract_fk_error(connection, driver);
    assert_eq!(
        fk_error.kind(),
        &berserk_database::ErrorKind::ForeignKeyViolation
    );

    {
        let mut tx = connection.begin(Default::default()).unwrap();
        contract_insert(&mut tx, driver, 10, "Rolled Back", None);
        tx.rollback().unwrap();
    }
    assert!(!contract_exists(connection, driver, 10));

    clean_database_contract(connection);
}

fn clean_database_contract(connection: &mut dyn Connection) {
    for table in [CONTRACT_CHILD, CONTRACT_PARENT] {
        connection
            .execute(&Statement::new(format!("DROP TABLE IF EXISTS {table}")))
            .unwrap();
    }
}

fn placeholder(driver: Driver, index: usize) -> String {
    match driver {
        Driver::Postgres => format!("${index}"),
        Driver::MySql | Driver::Sqlite => "?".to_owned(),
    }
}

fn contract_insert(
    connection: &mut dyn Connection,
    driver: Driver,
    id: i64,
    name: &str,
    optional: Option<&str>,
) {
    let sql = format!(
        "INSERT INTO {CONTRACT_PARENT} (id, name, optional_text) VALUES ({}, {}, {})",
        placeholder(driver, 1),
        placeholder(driver, 2),
        placeholder(driver, 3)
    );
    let optional_value = optional
        .map(berserk_database::Value::from)
        .unwrap_or(berserk_database::Value::Null);
    connection
        .execute(&Statement::new(sql).bind(id).bind(name).bind(optional_value))
        .unwrap();
}

fn contract_insert_error(
    connection: &mut dyn Connection,
    driver: Driver,
    id: i64,
    name: &str,
    optional: Option<&str>,
) -> berserk_database::DatabaseError {
    let sql = format!(
        "INSERT INTO {CONTRACT_PARENT} (id, name, optional_text) VALUES ({}, {}, {})",
        placeholder(driver, 1),
        placeholder(driver, 2),
        placeholder(driver, 3)
    );
    let optional_value = optional
        .map(berserk_database::Value::from)
        .unwrap_or(berserk_database::Value::Null);
    connection
        .execute(&Statement::new(sql).bind(id).bind(name).bind(optional_value))
        .unwrap_err()
}

fn contract_select(connection: &mut dyn Connection, driver: Driver) -> Vec<berserk_database::Row> {
    let sql = format!(
        "SELECT id, name, optional_text FROM {CONTRACT_PARENT} WHERE id >= {} ORDER BY id DESC LIMIT 2 OFFSET 1",
        placeholder(driver, 1)
    );
    connection.query(&Statement::new(sql).bind(1_i64)).unwrap()
}

fn contract_null_error(
    connection: &mut dyn Connection,
    driver: Driver,
    id: i64,
) -> berserk_database::DatabaseError {
    let sql = format!(
        "INSERT INTO {CONTRACT_PARENT} (id, name) VALUES ({}, NULL)",
        placeholder(driver, 1)
    );
    connection
        .execute(&Statement::new(sql).bind(id))
        .unwrap_err()
}

fn contract_fk_error(
    connection: &mut dyn Connection,
    driver: Driver,
) -> berserk_database::DatabaseError {
    let sql = format!(
        "INSERT INTO {CONTRACT_CHILD} (id, parent_id, label) VALUES ({}, {}, {})",
        placeholder(driver, 1),
        placeholder(driver, 2),
        placeholder(driver, 3)
    );
    connection
        .execute(&Statement::new(sql).bind(1_i64).bind(999_i64).bind("orphan"))
        .unwrap_err()
}

fn contract_exists(connection: &mut dyn Connection, driver: Driver, id: i64) -> bool {
    let sql = format!(
        "SELECT id FROM {CONTRACT_PARENT} WHERE id = {}",
        placeholder(driver, 1)
    );
    !connection
        .query(&Statement::new(sql).bind(id))
        .unwrap()
        .is_empty()
}

fn integer_eq(value: Option<&berserk_database::Value>, expected: i64) -> bool {
    matches!(value, Some(berserk_database::Value::I64(value)) if *value == expected)
        || matches!(value, Some(berserk_database::Value::U64(value)) if *value == expected as u64)
}

/// Cross-backend transaction contract: commit, rollback, multi-statement atomicity,
/// constraint rollback, and connection usability after completion.
pub fn run_live_transaction_contract(connection: &mut dyn Connection) {
    const TABLE: &str = "berserk_transaction_contract";
    connection
        .execute(&Statement::new(format!("DROP TABLE IF EXISTS {TABLE}")))
        .unwrap();
    connection
        .execute(&Statement::new(format!(
            "CREATE TABLE {TABLE} (id BIGINT PRIMARY KEY, value VARCHAR(255) NOT NULL UNIQUE)"
        )))
        .unwrap();
    let driver = connection.driver();

    {
        let mut tx = connection.begin(Default::default()).unwrap();
        transaction_insert(&mut *tx, driver, TABLE, 1, "committed").unwrap();
        tx.commit().unwrap();
    }
    assert!(transaction_exists(connection, driver, TABLE, 1));

    {
        let mut tx = connection.begin(Default::default()).unwrap();
        transaction_insert(&mut *tx, driver, TABLE, 2, "rolled-back").unwrap();
        transaction_insert(&mut *tx, driver, TABLE, 3, "also-rolled-back").unwrap();
        tx.rollback().unwrap();
    }
    assert!(!transaction_exists(connection, driver, TABLE, 2));
    assert!(!transaction_exists(connection, driver, TABLE, 3));

    {
        let mut tx = connection.begin(Default::default()).unwrap();
        transaction_insert(&mut *tx, driver, TABLE, 4, "duplicate").unwrap();
        let error = transaction_insert(&mut *tx, driver, TABLE, 5, "duplicate").unwrap_err();
        assert_eq!(error.kind(), &berserk_database::ErrorKind::UniqueViolation);
        tx.rollback().unwrap();
    }
    assert!(!transaction_exists(connection, driver, TABLE, 4));
    assert!(!transaction_exists(connection, driver, TABLE, 5));

    // A completed transaction must release the connection for ordinary work.
    contract_execute_insert(connection, driver, TABLE, 6, "after-transaction").unwrap();
    assert!(transaction_exists(connection, driver, TABLE, 6));

    connection
        .execute(&Statement::new(format!("DROP TABLE {TABLE}")))
        .unwrap();
}

fn transaction_insert(
    tx: &mut dyn berserk_database::Transaction,
    driver: Driver,
    table: &str,
    id: i64,
    value: &str,
) -> berserk_database::Result<berserk_database::Execution> {
    let sql = format!(
        "INSERT INTO {table} (id, value) VALUES ({}, {})",
        placeholder(driver, 1),
        placeholder(driver, 2)
    );
    tx.execute(&Statement::new(sql).bind(id).bind(value))
}

fn contract_execute_insert(
    connection: &mut dyn Connection,
    driver: Driver,
    table: &str,
    id: i64,
    value: &str,
) -> berserk_database::Result<berserk_database::Execution> {
    let sql = format!(
        "INSERT INTO {table} (id, value) VALUES ({}, {})",
        placeholder(driver, 1),
        placeholder(driver, 2)
    );
    connection.execute(&Statement::new(sql).bind(id).bind(value))
}

fn transaction_exists(
    connection: &mut dyn Connection,
    driver: Driver,
    table: &str,
    id: i64,
) -> bool {
    let sql = format!(
        "SELECT id FROM {table} WHERE id = {}",
        placeholder(driver, 1)
    );
    !connection
        .query(&Statement::new(sql).bind(id))
        .unwrap()
        .is_empty()
}
