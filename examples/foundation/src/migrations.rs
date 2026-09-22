use berserk::database::{
    migrations::{
        Column, ForeignAction, ForeignKey, Index, Migration, MigrationPlan, MigrationRunner, Table,
        Unique,
    },
    Connection, Driver, Result, Statement,
};

pub struct CreateFoundationSchema;

impl Migration for CreateFoundationSchema {
    fn name(&self) -> &'static str {
        "202609220001_create_foundation_schema"
    }

    fn up(&self, driver: Driver) -> Result<Vec<Statement>> {
        MigrationPlan::new()
            .create(
                Table::create("users")
                    .columns([
                        Column::id(),
                        Column::string("name"),
                        Column::string("email"),
                    ])
                    .uniques([Unique::new(["email"])]),
            )
            .create(
                Table::create("roles")
                    .columns([Column::id(), Column::string("name")])
                    .uniques([Unique::new(["name"])]),
            )
            .create(
                Table::create("posts")
                    .columns([
                        Column::id(),
                        Column::big_integer("user_id"),
                        Column::string("title"),
                    ])
                    .indexes([Index::new(["user_id"])])
                    .foreign_keys([
                        ForeignKey::new(["user_id"])
                            .references("users", ["id"])
                            .on_delete(ForeignAction::Cascade),
                    ]),
            )
            .create(
                Table::create("profiles")
                    .columns([Column::id(), Column::big_integer("user_id")])
                    .uniques([Unique::new(["user_id"])])
                    .foreign_keys([
                        ForeignKey::new(["user_id"])
                            .references("users", ["id"])
                            .on_delete(ForeignAction::Cascade),
                    ]),
            )
            .create(
                Table::create("role_user")
                    .columns([
                        Column::big_integer("user_id"),
                        Column::big_integer("role_id"),
                    ])
                    .primary(["user_id", "role_id"])
                    .indexes([Index::new(["role_id"])])
                    .foreign_keys([
                        ForeignKey::new(["user_id"])
                            .references("users", ["id"])
                            .on_delete(ForeignAction::Cascade),
                        ForeignKey::new(["role_id"])
                            .references("roles", ["id"])
                            .on_delete(ForeignAction::Cascade),
                    ]),
            )
            .compile(driver)
    }

    fn down(&self, driver: Driver) -> Result<Vec<Statement>> {
        MigrationPlan::new()
            .table(Table::drop("role_user"))
            .table(Table::drop("profiles"))
            .table(Table::drop("posts"))
            .table(Table::drop("roles"))
            .table(Table::drop("users"))
            .compile(driver)
    }
}

pub fn migrate(connection: &mut dyn Connection) -> Result<()> {
    let runner = MigrationRunner::new([&CreateFoundationSchema as &dyn Migration])?;
    runner.migrate(connection)?;
    Ok(())
}
