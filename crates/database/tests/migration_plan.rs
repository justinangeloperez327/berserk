use berserk_database::{
    migrations::{Column, Index, MigrationPlan, Table},
    Driver,
};

#[test]
fn migration_plan_compiles_create_indexes_and_drop() {
    let plan = MigrationPlan::new()
        .create(
            Table::create("users")
                .columns([Column::id(), Column::string("email")])
                .indexes([Index::new(["email"]).unique()]),
        )
        .table(Table::drop("users"));

    let statements = plan.compile(Driver::Postgres).unwrap();

    assert_eq!(statements.len(), 3);
    assert!(statements[0].sql().starts_with("CREATE TABLE"));
    assert!(statements[1].sql().starts_with("CREATE UNIQUE INDEX"));
    assert_eq!(statements[2].sql(), "DROP TABLE \"users\"");
}

#[test]
fn migration_plan_compiles_alter_operations_in_order() {
    let plan = MigrationPlan::new().alter(
        Table::alter("users")
            .add([Column::string("phone").nullable()])
            .rename("phone", "mobile"),
    );

    let statements = plan.compile(Driver::MySql).unwrap();

    assert_eq!(statements.len(), 2);
    assert!(statements[0].sql().contains("ADD COLUMN"));
    assert!(statements[1].sql().contains("RENAME COLUMN"));
}
