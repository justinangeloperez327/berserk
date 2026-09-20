use berserk_database::{Direction, Driver, Query, Value};

#[test]
fn select_compiles_for_postgres_with_stable_binding_order() {
    let statement = Query::table("users")
        .select(["users.id", "profiles.name"])
        .left_join("profiles", "users.id", "profiles.user_id")
        .where_("users.active", "=", true)
        .where_in("users.id", [10_i64, 20_i64])
        .order_by("profiles.name", Direction::Asc)
        .limit(25)
        .offset(5)
        .to_statement(Driver::Postgres)
        .unwrap();

    assert_eq!(statement.sql(), "SELECT \"users\".\"id\", \"profiles\".\"name\" FROM \"users\" LEFT JOIN \"profiles\" ON \"users\".\"id\" = \"profiles\".\"user_id\" WHERE \"users\".\"active\" = $1 AND \"users\".\"id\" IN ($2, $3) ORDER BY \"profiles\".\"name\" ASC LIMIT 25 OFFSET 5");
    assert_eq!(
        statement.bindings(),
        &[Value::Bool(true), Value::I64(10), Value::I64(20)]
    );
}

#[test]
fn mysql_uses_question_mark_placeholders() {
    let statement = Query::table("users")
        .where_("id", ">", 4_i64)
        .or_where_null("deleted_at")
        .to_statement(Driver::MySql)
        .unwrap();
    assert_eq!(
        statement.sql(),
        "SELECT * FROM `users` WHERE `id` > ? OR `deleted_at` IS NULL"
    );
}

#[test]
fn insert_and_update_put_values_before_filter_bindings() {
    let insert = Query::table("users")
        .insert(vec![
            ("name", Value::from("Ada")),
            ("active", Value::from(true)),
        ])
        .to_statement(Driver::Sqlite)
        .unwrap();
    assert_eq!(
        insert.sql(),
        "INSERT INTO \"users\" (\"name\", \"active\") VALUES (?, ?)"
    );

    let update = Query::table("users")
        .where_("id", "=", 7_i64)
        .update(vec![
            ("name", Value::from("Grace")),
            ("active", Value::from(false)),
        ])
        .to_statement(Driver::Postgres)
        .unwrap();
    assert_eq!(
        update.sql(),
        "UPDATE \"users\" SET \"name\" = $1, \"active\" = $2 WHERE \"id\" = $3"
    );
    assert_eq!(
        update.bindings(),
        &[
            Value::Text("Grace".into()),
            Value::Bool(false),
            Value::I64(7)
        ]
    );
}

#[test]
fn destructive_queries_require_filters_or_explicit_override() {
    assert!(Query::table("users")
        .delete()
        .to_statement(Driver::Sqlite)
        .is_err());
    assert_eq!(
        Query::table("users")
            .delete()
            .allow_all()
            .to_statement(Driver::Sqlite)
            .unwrap()
            .sql(),
        "DELETE FROM \"users\""
    );
    assert!(Query::table("users")
        .update([("active", false)])
        .to_statement(Driver::Sqlite)
        .is_err());
}

#[test]
fn identifiers_and_operators_cannot_inject_sql() {
    assert!(Query::table("users; drop table users")
        .to_statement(Driver::Postgres)
        .is_err());
    assert!(Query::table("users")
        .where_("id", "= 1; --", 1_i64)
        .to_statement(Driver::Postgres)
        .is_err());
}

#[test]
fn empty_in_lists_have_deterministic_boolean_sql() {
    let statement = Query::table("users")
        .where_in("id", Vec::<i64>::new())
        .to_statement(Driver::Sqlite)
        .unwrap();
    assert_eq!(statement.sql(), "SELECT * FROM \"users\" WHERE 1 = 0");
    assert!(statement.bindings().is_empty());
}

#[test]
fn raw_queries_remain_explicit_and_bound() {
    let raw = Query::raw("select * from users where id = ?").bind(9_i64);
    assert_eq!(raw.statement().sql(), "select * from users where id = ?");
    assert_eq!(raw.statement().bindings(), &[Value::I64(9)]);
}

#[test]
fn between_predicates_keep_bounds_bound_and_ordered() {
    let statement = Query::table("orders")
        .where_between("total", 100_i64, 500_i64)
        .or_where_not_between("created_at", "2026-01-01", "2026-01-31")
        .to_statement(Driver::Postgres)
        .unwrap();

    assert_eq!(
        statement.sql(),
        "SELECT * FROM \"orders\" WHERE \"total\" BETWEEN $1 AND $2 OR \"created_at\" NOT BETWEEN $3 AND $4"
    );
    assert_eq!(
        statement.bindings(),
        &[
            Value::I64(100),
            Value::I64(500),
            Value::Text("2026-01-01".into()),
            Value::Text("2026-01-31".into()),
        ]
    );

    assert!(Query::table("orders")
        .where_between("total", Value::Null, 500_i64)
        .to_statement(Driver::Sqlite)
        .is_err());
}
