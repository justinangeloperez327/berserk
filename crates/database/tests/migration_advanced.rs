use berserk_database::{
    migrations::{
        compile_alter, compile_comments, compile_create, compile_rebuild, Check, Column, Table,
        Unique,
    },
    Driver,
};

#[test]
fn create_table_compiles_advanced_constraints() {
    let table = Table::create("products")
        .comment("Sellable products")
        .columns([
            Column::id(),
            Column::enum_("status", ["draft", "active"]),
            Column::decimal("price", 10, 2),
            Column::string("slug").comment("Public slug"),
            Column::integer("price_cents")
                .generated_stored("price * 100"),
        ])
        .uniques([Unique::new(["slug"]).named("products_slug_unique")])
        .checks([Check::new("products_price_positive", "price >= 0")]);

    let postgres = compile_create(&table, Driver::Postgres).unwrap();
    assert!(postgres
        .sql()
        .contains("CONSTRAINT \"products_slug_unique\" UNIQUE (\"slug\")"));
    assert!(postgres
        .sql()
        .contains("CONSTRAINT \"products_price_positive\" CHECK (price >= 0)"));
    assert!(postgres
        .sql()
        .contains("\"status\" VARCHAR(255) NOT NULL CHECK (\"status\" IN ('draft', 'active'))"));
    assert!(postgres
        .sql()
        .contains("GENERATED ALWAYS AS (price * 100) STORED"));

    let comments = compile_comments(&table, Driver::Postgres).unwrap();
    assert_eq!(comments.len(), 2);

    let mysql = compile_create(&table, Driver::MySql).unwrap();
    assert!(mysql.sql().contains("COMMENT='Sellable products'"));
    assert!(mysql.sql().contains("COMMENT 'Public slug'"));

    assert!(compile_comments(&table, Driver::Sqlite).is_err());
}

#[test]
fn alter_table_compiles_defaults_indexes_and_constraints() {
    let table = Table::alter("users")
        .rename_index("users_email_idx", "users_email_index")
        .set_default("active", true)
        .drop_default("nickname")
        .add_check(Check::new("users_age_positive", "age >= 0"))
        .drop_check("users_old_check")
        .add_unique(Unique::new(["email"]).named("users_email_unique"))
        .drop_unique("users_legacy_unique");

    let postgres = compile_alter(&table, Driver::Postgres).unwrap();
    assert_eq!(postgres.len(), 7);
    assert!(postgres[0].sql().contains("ALTER INDEX"));
    assert!(postgres[1].sql().contains("SET DEFAULT TRUE"));
    assert!(postgres[2].sql().contains("DROP DEFAULT"));
    assert!(postgres[3].sql().contains("ADD CONSTRAINT"));
}

#[test]
fn sqlite_rebuild_compiles_copy_swap_and_indexes() {
    let replacement = Table::create("users").columns([
        Column::id(),
        Column::string("full_name"),
        Column::string("email"),
    ]);
    let rebuild = Table::rebuild("users", replacement)
        .copy([("id", "id"), ("name", "full_name"), ("email", "email")]);

    let statements = compile_rebuild(&rebuild, Driver::Sqlite).unwrap();
    assert_eq!(statements.len(), 4);
    assert!(statements[0].sql().contains("CREATE TABLE \"__br_users\""));
    assert!(statements[1].sql().contains("INSERT INTO \"__br_users\""));
    assert_eq!(statements[2].sql(), "DROP TABLE \"users\"");
    assert_eq!(
        statements[3].sql(),
        "ALTER TABLE \"__br_users\" RENAME TO \"users\""
    );
}

#[test]
fn custom_column_types_are_an_explicit_escape_hatch() {
    let table = Table::create("events").columns([
        Column::id(),
        Column::custom("location", "GEOGRAPHY(POINT,4326)"),
    ]);

    let statement = compile_create(&table, Driver::Postgres).unwrap();
    assert!(statement.sql().contains("GEOGRAPHY(POINT,4326)"));
}
