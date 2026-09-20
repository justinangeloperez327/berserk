use berserk_database::migrations::{Column, ColumnType, Table};

#[test]
fn create_table_uses_declarative_columns() {
    let table = Table::create("users").columns([
        Column::id(),
        Column::string("name"),
        Column::string("email").unique(),
        Column::boolean("active").default(true),
        Column::timestamp("created_at"),
        Column::timestamp("updated_at"),
    ]);

    table.validate().unwrap();

    assert_eq!(table.name(), "users");
    assert_eq!(table.column_definitions().len(), 6);
    assert_eq!(table.column_definitions()[0].name(), "id");
    assert_eq!(
        table.column_definitions()[1].kind(),
        &ColumnType::String(None)
    );
}

#[test]
fn create_table_rejects_duplicate_columns() {
    let table = Table::create("users").columns([
        Column::id(),
        Column::string("email"),
        Column::string("email"),
    ]);

    assert!(table.validate().is_err());
}

#[test]
fn column_definitions_validate_lengths_and_decimals() {
    assert!(Table::create("users")
        .columns([Column::string("name").length(0)])
        .validate()
        .is_err());
    assert!(Table::create("products")
        .columns([Column::decimal("price", 2, 3)])
        .validate()
        .is_err());
}
