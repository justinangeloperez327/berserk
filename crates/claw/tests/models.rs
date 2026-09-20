use berserk_database::{
    Capabilities, Column, Connection, DatabaseError, Driver, ErrorKind, Execution, Transaction,
    TransactionOptions,
};
use claw_orm::{
    field, BelongsTo, Direction, HasMany, HasOne, Model, Result, Row, Statement, Value,
};

#[derive(Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            active: field(row, "active")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

#[derive(Debug, PartialEq)]
struct Post {
    id: u64,
    user_id: u64,
    title: String,
}

impl Model for Post {
    const TABLE: &'static str = "posts";

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            user_id: field(row, "user_id")?,
            title: field(row, "title")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

fn user_row(id: u64, name: &str) -> Row {
    Row::new(vec![
        Column::new("id", id),
        Column::new("name", name),
        Column::new("active", true),
    ])
    .unwrap()
}

fn post_row(id: u64, user_id: u64, title: &str) -> Row {
    Row::new(vec![
        Column::new("id", id),
        Column::new("user_id", user_id),
        Column::new("title", title),
    ])
    .unwrap()
}

#[derive(Default)]
struct FakeConnection {
    rows: Vec<Row>,
    statements: Vec<Statement>,
    executed: Vec<Statement>,
}

impl FakeConnection {
    fn with_rows(rows: Vec<Row>) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }
}

impl Connection for FakeConnection {
    fn driver(&self) -> Driver {
        Driver::Sqlite
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }

    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        let last_insert_id = statement.sql().starts_with("INSERT ").then_some(42);
        self.executed.push(statement.clone());
        Ok(Execution {
            affected_rows: 1,
            last_insert_id,
        })
    }

    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.statements.push(statement.clone());
        Ok(std::mem::take(&mut self.rows))
    }

    fn begin(&mut self, _options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        Err(DatabaseError::new(
            ErrorKind::Transaction,
            "not supported by fake",
        ))
    }

    fn ping(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn model_queries_decode_rows_and_keep_execution_explicit() {
    fn active(query: berserk_database::Query) -> berserk_database::Query {
        query.where_("active", "=", true)
    }

    let query = User::query().scope(active).order_by("name", Direction::Asc);
    let statement = query.to_statement(Driver::Postgres).unwrap();
    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"active\" = $1 ORDER BY \"name\" ASC"
    );

    let mut connection = FakeConnection::with_rows(vec![user_row(7, "Ada")]);
    let users = query.get_on(&mut connection).unwrap();
    assert_eq!(
        users,
        vec![User {
            id: 7,
            name: "Ada".into(),
            active: true,
        }]
    );
    assert_eq!(connection.statements.len(), 1);
}

#[test]
fn model_static_query_entry_points_build_typed_queries() {
    let query = User::where_op("active", "=", true)
        .where_not_null("name")
        .order_by("name", Direction::Desc)
        .limit(10);
    let statement = query.to_statement(Driver::Postgres).unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"active\" = $1 AND \"name\" IS NOT NULL ORDER BY \"name\" DESC LIMIT 10"
    );
    assert_eq!(statement.bindings(), &[Value::Bool(true)]);
}

#[test]
fn create_uses_the_model_table_and_keeps_values_bound() {
    let mut connection = FakeConnection::default();
    let execution = User::create_on(
        &mut connection,
        [("name", Value::from("Ada")), ("active", Value::from(true))],
    )
    .unwrap();

    assert_eq!(execution.affected_rows, 1);
    assert_eq!(execution.last_insert_id, Some(42));
    assert_eq!(connection.executed.len(), 1);
    assert_eq!(
        connection.executed[0].sql(),
        "INSERT INTO \"users\" (\"name\", \"active\") VALUES (?, ?)"
    );
    assert_eq!(
        connection.executed[0].bindings(),
        &[Value::Text("Ada".into()), Value::Bool(true)]
    );
}

#[test]
fn filtered_update_and_delete_use_bound_model_queries() {
    let mut connection = FakeConnection::default();

    let updated = User::where_op("id", "=", 7_u64)
        .update_on(&mut connection, [("name", Value::from("Grace"))])
        .unwrap();
    assert_eq!(updated.affected_rows, 1);
    assert_eq!(updated.last_insert_id, None);
    assert_eq!(
        connection.executed[0].sql(),
        "UPDATE \"users\" SET \"name\" = ? WHERE \"id\" = ?"
    );
    assert_eq!(
        connection.executed[0].bindings(),
        &[Value::Text("Grace".into()), Value::U64(7)]
    );

    let deleted = User::where_op("id", "=", 7_u64)
        .delete_on(&mut connection)
        .unwrap();
    assert_eq!(deleted.affected_rows, 1);
    assert_eq!(deleted.last_insert_id, None);
    assert_eq!(
        connection.executed[1].sql(),
        "DELETE FROM \"users\" WHERE \"id\" = ?"
    );
    assert_eq!(connection.executed[1].bindings(), &[Value::U64(7)]);
}

#[test]
fn instance_update_and_delete_filter_by_the_model_key() {
    let user = User {
        id: 7,
        name: "Ada".into(),
        active: true,
    };
    let mut connection = FakeConnection::default();

    let updated = user
        .update_on(&mut connection, [("name", Value::from("Grace"))])
        .unwrap();
    assert_eq!(updated.affected_rows, 1);
    assert_eq!(user.name, "Ada");
    assert_eq!(
        connection.executed[0].sql(),
        "UPDATE \"users\" SET \"name\" = ? WHERE \"id\" = ?"
    );
    assert_eq!(
        connection.executed[0].bindings(),
        &[Value::Text("Grace".into()), Value::U64(7)]
    );

    let deleted = user.delete_on(&mut connection).unwrap();
    assert_eq!(deleted.affected_rows, 1);
    assert_eq!(
        connection.executed[1].sql(),
        "DELETE FROM \"users\" WHERE \"id\" = ?"
    );
    assert_eq!(connection.executed[1].bindings(), &[Value::U64(7)]);
}

#[test]
fn unfiltered_model_mutations_are_rejected_before_execution() {
    let mut connection = FakeConnection::default();

    let update_error = User::query()
        .update_on(&mut connection, [("active", Value::from(false))])
        .unwrap_err();
    assert!(matches!(update_error.kind(), ErrorKind::Query));

    let delete_error = User::query().delete_on(&mut connection).unwrap_err();
    assert!(matches!(delete_error.kind(), ErrorKind::Query));
    assert!(connection.executed.is_empty());
}

#[test]
fn find_uses_the_declared_primary_key() {
    let mut connection = FakeConnection::with_rows(vec![user_row(9, "Lin")]);
    let user = User::find_on(&mut connection, 9_u64).unwrap().unwrap();
    assert_eq!(user.id, 9);
    assert_eq!(connection.statements[0].bindings(), &[Value::U64(9)]);
}

#[test]
fn destroy_filters_by_the_declared_primary_key() {
    let mut connection = FakeConnection::default();
    let execution = User::destroy_on(&mut connection, 9_u64).unwrap();

    assert_eq!(execution.affected_rows, 1);
    assert_eq!(execution.last_insert_id, None);
    assert_eq!(connection.executed.len(), 1);
    assert_eq!(
        connection.executed[0].sql(),
        "DELETE FROM \"users\" WHERE \"id\" = ?"
    );
    assert_eq!(connection.executed[0].bindings(), &[Value::U64(9)]);
}

#[test]
fn field_casting_is_strict_and_nullable_fields_are_explicit() {
    let row = Row::new(vec![Column::new("name", Value::Null)]).unwrap();
    assert_eq!(field::<Option<String>>(&row, "name").unwrap(), None);
    assert!(matches!(
        field::<String>(&row, "name").unwrap_err().kind(),
        ErrorKind::Decode
    ));
    assert!(matches!(
        field::<u64>(&row, "missing").unwrap_err().kind(),
        ErrorKind::Decode
    ));
}

#[test]
fn has_many_eager_loads_once_and_groups_by_foreign_key() {
    let users = vec![
        User {
            id: 1,
            name: "A".into(),
            active: true,
        },
        User {
            id: 2,
            name: "B".into(),
            active: true,
        },
    ];
    let relation = HasMany::<User, Post>::new("user_id", User::key, |post| post.user_id.into());
    let mut connection =
        FakeConnection::with_rows(vec![post_row(10, 1, "One"), post_row(11, 1, "Two")]);
    let loaded = relation.load_on(&mut connection, &users).unwrap();

    assert_eq!(connection.statements.len(), 1);
    assert_eq!(loaded.get(&Value::U64(1)).unwrap().len(), 2);
    assert!(loaded.get(&Value::U64(2)).is_none());
}

#[test]
fn belongs_to_skips_null_keys_and_has_one_rejects_duplicates() {
    let posts = vec![Post {
        id: 1,
        user_id: 8,
        title: "Post".into(),
    }];
    let owner = BelongsTo::<Post, User>::new("id", |post| Some(post.user_id.into()), User::key);
    let mut owner_connection = FakeConnection::with_rows(vec![user_row(8, "Owner")]);
    let loaded = owner.load_on(&mut owner_connection, &posts).unwrap();
    assert_eq!(loaded.get(&Value::U64(8)).unwrap()[0].name, "Owner");

    let one = HasOne::<User, Post>::new("user_id", User::key, |post| post.user_id.into());
    let mut duplicate_connection =
        FakeConnection::with_rows(vec![post_row(1, 8, "A"), post_row(2, 8, "B")]);
    let error = one
        .load_on(
            &mut duplicate_connection,
            &[User {
                id: 8,
                name: "Owner".into(),
                active: true,
            }],
        )
        .unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::Decode));
}

#[test]
fn empty_parent_sets_do_not_contact_the_database() {
    let relation = HasMany::<User, Post>::new("user_id", User::key, |post| post.user_id.into());
    let mut connection = FakeConnection::default();
    let loaded = relation.load_on(&mut connection, &[]).unwrap();
    assert!(loaded.is_empty());
    assert!(connection.statements.is_empty());
}
