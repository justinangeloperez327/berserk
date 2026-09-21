use berserk_database::{drivers::sqlite::SqliteConnection, Query};
use claw_orm::{field, BelongsToMany, Connection, ErrorKind, Model, Result, Row, Statement, Value};

#[derive(Debug)]
struct User(u64);
#[derive(Debug)]
struct Role(u64);
impl Model for User {
    const TABLE: &'static str = "berserk_claw_users";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self(field(row, "id")?))
    }
    fn key(&self) -> Value {
        self.0.into()
    }
}
impl Model for Role {
    const TABLE: &'static str = "berserk_claw_roles";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self(field(row, "id")?))
    }
    fn key(&self) -> Value {
        self.0.into()
    }
}
fn roles() -> BelongsToMany<User, Role> {
    BelongsToMany::new(
        "berserk_claw_role_user",
        "user_id",
        "role_id",
        User::key,
        Role::key,
    )
}

// Dedicated integration-test database only; names are isolated from applications/examples.
fn clean(c: &mut dyn Connection) {
    for table in [
        "berserk_claw_role_user",
        "berserk_claw_roles",
        "berserk_claw_users",
    ] {
        c.execute(&Statement::new(format!("DROP TABLE IF EXISTS {table}")))
            .unwrap();
    }
}
fn contract(c: &mut dyn Connection) {
    clean(c);
    for sql in [
        "CREATE TABLE berserk_claw_users (id BIGINT PRIMARY KEY)",
        "CREATE TABLE berserk_claw_roles (id BIGINT PRIMARY KEY)",
        "CREATE TABLE berserk_claw_role_user (user_id BIGINT NOT NULL, role_id BIGINT NOT NULL, UNIQUE(user_id, role_id), FOREIGN KEY(user_id) REFERENCES berserk_claw_users(id), FOREIGN KEY(role_id) REFERENCES berserk_claw_roles(id))",
        "INSERT INTO berserk_claw_users VALUES (1), (2)",
        "INSERT INTO berserk_claw_roles VALUES (10), (11), (12)",
    ] { c.execute(&Statement::new(sql)).unwrap(); }
    let relation = roles();
    relation.attach_many_on(c, &User(1), [10, 11, 11]).unwrap();
    relation.attach_on(c, &User(2), 10).unwrap();
    assert_eq!(
        relation.attach_on(c, &User(1), 10).unwrap_err().kind(),
        &ErrorKind::Constraint
    );
    assert_eq!(
        relation.sync_on(c, &User(1), [12, 999]).unwrap_err().kind(),
        &ErrorKind::Constraint
    );
    let loaded = User::query().with(roles()).get_on(c).unwrap();
    assert_eq!(loaded.relations.get(&Value::U64(1)).unwrap().len(), 2);
    assert_eq!(loaded.relations.get(&Value::U64(2)).unwrap()[0].0, 10);
    let result = relation.sync_on(c, &User(1), [11, 12, 12]).unwrap();
    assert_eq!(result.attached, [Value::U64(12)]);
    assert_eq!(result.detached, [Value::U64(10)]);
    assert_eq!(
        relation.query_for(&User(1)).unwrap().count_on(c).unwrap(),
        2
    );
    let page = User::query()
        .order_by("id", claw_orm::Direction::Asc)
        .with(roles())
        .paginate_on(c, 2, 1)
        .unwrap();
    assert_eq!(page.page.total(), 2);
    assert_eq!(page.relations.get(&Value::U64(2)).unwrap()[0].0, 10);
    relation.detach_many_on(c, &User(1), [11, 12]).unwrap();
    assert_eq!(Query::table("berserk_claw_role_user").count(c).unwrap(), 1);
    relation.detach_all_on(c, &User(2)).unwrap();
    assert_eq!(Query::table("berserk_claw_role_user").count(c).unwrap(), 0);
    clean(c);
}

#[test]
fn sqlite_relationship_contract() {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.execute(&Statement::new("PRAGMA foreign_keys = ON"))
        .unwrap();
    contract(&mut c);
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn database_url(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(url) => Some(url),
        Err(_) if std::env::var("BERSERK_REQUIRE_LIVE_DATABASES").as_deref() == Ok("1") => {
            panic!("{name} is required for live relationship tests")
        }
        Err(_) => {
            eprintln!("Skipping live relationship test: {name} is not set");
            None
        }
    }
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_relationship_contract() {
    if let Some(url) = database_url("FRAMEWORK_POSTGRES_TEST_URL") {
        let mut c =
            berserk_database::drivers::postgres::PostgresConnection::connect_no_tls(&url).unwrap();
        contract(&mut c);
    }
}

#[cfg(feature = "mysql")]
#[test]
fn mysql_relationship_contract() {
    if let Some(url) = database_url("FRAMEWORK_MYSQL_TEST_URL") {
        let mut c = berserk_database::drivers::mysql::MySqlConnection::connect(&url).unwrap();
        contract(&mut c);
    }
}
