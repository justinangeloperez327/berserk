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
        .where_("email", "=", "a@example.com")
        .limit(1)
        .to_statement(Driver::Sqlite)
        .unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"email\" = ? LIMIT 1"
    );
    assert_eq!(statement.bindings(), &[Value::Text("a@example.com".into())]);
}
