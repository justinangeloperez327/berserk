use berserk_database::{
    migrations::{
        compile_alter, compile_table_operation, AlterOperation, Column, Table, TableOperation,
    },
    Driver,
};

#[test]
fn alter_table_builds_explicit_operations() {
    let alter = Table::alter("users")
        .add([Column::string("phone").nullable()])
        .rename("name", "full_name")
        .drop(["legacy"])
        .modify(Column::string("email").length(320));

    alter.validate().unwrap();
    assert_eq!(alter.operations().len(), 4);
    assert!(matches!(alter.operations()[0], AlterOperation::Add(_)));
}

#[test]
fn alter_table_compiles_for_postgres() {
    let statements = compile_alter(
        &Table::alter("users")
            .add([Column::string("phone").nullable()])
            .rename("name", "full_name")
            .drop(["legacy"]),
        Driver::Postgres,
    )
    .unwrap();

    assert_eq!(
        statements[0].sql(),
        "ALTER TABLE \"users\" ADD COLUMN \"phone\" VARCHAR(255)"
    );
    assert_eq!(
        statements[1].sql(),
        "ALTER TABLE \"users\" RENAME COLUMN \"name\" TO \"full_name\""
    );
    assert_eq!(
        statements[2].sql(),
        "ALTER TABLE \"users\" DROP COLUMN \"legacy\""
    );
}

#[test]
fn sqlite_rejects_direct_column_modification() {
    let result = compile_alter(
        &Table::alter("users").modify(Column::string("email").length(320)),
        Driver::Sqlite,
    );
    assert!(result.is_err());
}

#[test]
fn table_rename_and_drop_compile() {
    let rename = compile_table_operation(&Table::rename("users", "accounts"), Driver::MySql).unwrap();
    let drop = compile_table_operation(&Table::drop_if_exists("accounts"), Driver::Sqlite).unwrap();

    assert_eq!(rename.sql(), "ALTER TABLE `users` RENAME TO `accounts`");
    assert_eq!(drop.sql(), "DROP TABLE IF EXISTS \"accounts\"");

    assert!(matches!(
        Table::drop("users"),
        TableOperation::Drop {
            if_exists: false,
            ..
        }
    ));
}
