use claw_orm::{Driver, Model, Row, Value};

#[derive(Debug, PartialEq)]
struct User {
    id: u64,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> claw_orm::Result<Self> {
        Ok(Self {
            id: claw_orm::field(row, "id")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

#[test]
fn model_query_keeps_values_bound() {
    let statement = User::query()
        .where_op("email", "=", "a@example.com")
        .limit(1)
        .to_statement(Driver::Sqlite)
        .unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"email\" = ? LIMIT 1"
    );
    assert_eq!(statement.bindings(), &[Value::Text("a@example.com".into())]);
}


#[test]
fn model_query_exposes_between_and_or_variants() {
    let statement = User::where_between("id", 10_u64, 20_u64)
        .or_where_not_in("id", [30_u64, 40_u64])
        .or_where_null("deleted_at")
        .to_statement(Driver::Postgres)
        .unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"id\" BETWEEN $1 AND $2 OR \"id\" NOT IN ($3, $4) OR \"deleted_at\" IS NULL"
    );
    assert_eq!(
        statement.bindings(),
        &[
            Value::U64(10),
            Value::U64(20),
            Value::U64(30),
            Value::U64(40),
        ]
    );
}
