use berserk_database::{
    migrations::{compile_create, Column, Table},
    Driver,
};

fn users() -> berserk_database::migrations::CreateTable {
    Table::create("users").columns([
        Column::id(),
        Column::string("name"),
        Column::string("email").unique(),
        Column::boolean("active").default(true),
        Column::timestamp("created_at"),
        Column::timestamp("updated_at"),
    ])
}

#[test]
fn postgres_create_table_is_compiled() {
    let statement = compile_create(&users(), Driver::Postgres).unwrap();
    assert_eq!(
        statement.sql(),
        "CREATE TABLE \"users\" (\"id\" BIGSERIAL PRIMARY KEY, \"name\" VARCHAR(255) NOT NULL, \"email\" VARCHAR(255) NOT NULL UNIQUE, \"active\" BOOLEAN NOT NULL DEFAULT TRUE, \"created_at\" TIMESTAMP NOT NULL, \"updated_at\" TIMESTAMP NOT NULL)"
    );
}

#[test]
fn mysql_create_table_is_compiled() {
    let statement = compile_create(&users(), Driver::MySql).unwrap();
    assert_eq!(
        statement.sql(),
        "CREATE TABLE `users` (`id` BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, `name` VARCHAR(255) NOT NULL, `email` VARCHAR(255) NOT NULL UNIQUE, `active` BOOLEAN NOT NULL DEFAULT TRUE, `created_at` TIMESTAMP NOT NULL, `updated_at` TIMESTAMP NOT NULL)"
    );
}

#[test]
fn sqlite_create_table_is_compiled() {
    let statement = compile_create(&users(), Driver::Sqlite).unwrap();
    assert_eq!(
        statement.sql(),
        "CREATE TABLE \"users\" (\"id\" INTEGER PRIMARY KEY, \"name\" VARCHAR(255) NOT NULL, \"email\" VARCHAR(255) NOT NULL UNIQUE, \"active\" BOOLEAN NOT NULL DEFAULT TRUE, \"created_at\" TEXT NOT NULL, \"updated_at\" TEXT NOT NULL)"
    );
}

#[test]
fn current_timestamp_default_is_portable() {
    for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
        let table = Table::create("events").columns([
            Column::id(),
            Column::timestamp("created_at").default_current_timestamp(),
        ]);
        let statement = compile_create(&table, driver).unwrap();
        assert!(statement.sql().contains("DEFAULT CURRENT_TIMESTAMP"));
    }
}
