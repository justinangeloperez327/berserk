use claw_orm::{Direction, Driver, Model, Row, Value};

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
fn combined_query_preserves_predicate_and_binding_order_for_every_driver() {
    for driver in [Driver::Sqlite, Driver::MySql, Driver::Postgres] {
        let statement = User::where_("active", true)
            .where_in("role_id", [2_u64, 3])
            .or_where_between("score", 80_u64, 100)
            .where_not_null("email")
            .order_by("id", Direction::Desc)
            .limit(25)
            .offset(50)
            .to_statement(driver)
            .unwrap();

        let sql = statement.sql();
        assert!(sql.contains("active"));
        assert!(sql.contains("role_id"));
        assert!(sql.contains("score"));
        assert!(sql.contains("email"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.ends_with("LIMIT 25 OFFSET 50"));
        assert_eq!(
            statement.bindings(),
            &[
                Value::Bool(true),
                Value::U64(2),
                Value::U64(3),
                Value::U64(80),
                Value::U64(100)
            ]
        );
        if driver == Driver::Postgres {
            for marker in ["$1", "$2", "$3", "$4", "$5"] {
                assert!(sql.contains(marker));
            }
        } else {
            assert_eq!(sql.matches('?').count(), 5);
        }
    }
}

#[test]
fn empty_in_predicates_do_not_shift_later_bindings() {
    let statement = User::where_in("id", Vec::<u64>::new())
        .or_where("id", 7_u64)
        .where_not_in("id", Vec::<u64>::new())
        .to_statement(Driver::Postgres)
        .unwrap();
    assert_eq!(statement.bindings(), &[Value::U64(7)]);
    assert!(statement.sql().contains("$1"));
    assert!(!statement.sql().contains("$2"));
}

#[test]
fn scope_composes_without_changing_the_model_type() {
    let query = User::query()
        .scope(|q| q.where_("active", "=", true))
        .where_("id", 9_u64);
    let statement = query.to_statement(Driver::Sqlite).unwrap();
    assert_eq!(statement.bindings(), &[Value::Bool(true), Value::U64(9)]);
}
