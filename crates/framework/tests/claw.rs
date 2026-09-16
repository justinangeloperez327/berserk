#![cfg(feature = "claw")]

use framework::claw::{Driver, Result as DatabaseResult, Row, Value};
use framework::prelude::*;

struct User {
    id: u64,
}

impl Model for User {
    const TABLE: &'static str = "users";

    fn from_row(row: &Row) -> DatabaseResult<Self> {
        Ok(Self {
            id: framework::claw::field(row, "id")?,
        })
    }

    fn key(&self) -> Value {
        self.id.into()
    }
}

#[test]
fn claw_model_is_available_through_the_framework_prelude() {
    let statement = User::where_("active", "=", true)
        .to_statement(Driver::Postgres)
        .unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"users\" WHERE \"active\" = $1"
    );
    assert_eq!(statement.bindings(), &[Value::Bool(true)]);
}
