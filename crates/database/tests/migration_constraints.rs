use berserk_database::{
    migrations::{\n        compile_create, compile_indexes, Column, ForeignAction, ForeignKey, Index, Table,\n    },
    Driver,
};

#[test]
fn foreign_keys_compile_with_referential_actions() {
    let table = Table::create("posts")
        .columns([
            Column::id(),
            Column::big_integer("user_id"),
            Column::string("slug"),
        ])
        .foreign_keys([
            ForeignKey::new(["user_id"])
                .references("users", ["id"])
                .on_delete(ForeignAction::Cascade)
                .on_update(ForeignAction::Restrict),
        ]);

    let statement = compile_create(&table, Driver::Postgres).unwrap();
    assert!(statement.sql().contains(
        "FOREIGN KEY (\"user_id\") REFERENCES \"users\" (\"id\") ON DELETE CASCADE ON UPDATE RESTRICT"
    ));
}

#[test]
fn composite_and_unique_indexes_compile() {
    let table = Table::create("memberships")
        .columns([
            Column::id(),
            Column::big_integer("team_id"),
            Column::big_integer("user_id"),
        ])
        .indexes([
            Index::new(["team_id"]),
            Index::new(["team_id", "user_id"])
                .unique()
                .named("memberships_team_user_unique"),
        ]);

    let statements = compile_indexes(&table, Driver::Postgres).unwrap();
    assert_eq!(statements.len(), 2);
    assert_eq!(
        statements[0].sql(),
        "CREATE INDEX \"idx_memberships_team_id\" ON \"memberships\" (\"team_id\")"
    );
    assert_eq!(
        statements[1].sql(),
        "CREATE UNIQUE INDEX \"memberships_team_user_unique\" ON \"memberships\" (\"team_id\", \"user_id\")"
    );
}

#[test]
fn constraints_reject_unknown_local_columns() {
    let table = Table::create("posts")
        .columns([Column::id()])
        .foreign_keys([ForeignKey::new(["user_id"]).references("users", ["id"])]);

    assert!(table.validate().is_err());
}


#[test]
fn named_foreign_keys_and_composite_primary_keys_compile() {
    let table = Table::create("role_user")
        .columns([
            Column::big_integer("role_id"),
            Column::big_integer("user_id"),
        ])
        .primary(["role_id", "user_id"])
        .foreign_keys([
            ForeignKey::new(["user_id"])
                .references("users", ["id"])
                .named("role_user_user_fk"),
        ]);

    let statement = compile_create(&table, Driver::Postgres).unwrap();

    assert!(statement
        .sql()
        .contains("PRIMARY KEY (\"role_id\", \"user_id\")"));
    assert!(statement.sql().contains(
        "CONSTRAINT \"role_user_user_fk\" FOREIGN KEY (\"user_id\") REFERENCES \"users\" (\"id\")"
    ));
}
