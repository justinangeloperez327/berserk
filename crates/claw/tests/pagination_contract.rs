#![cfg(feature = "sqlite")]

use berserk_database::{drivers::sqlite::SqliteConnection, scope::DatabaseScope, Connection, Database, ErrorKind, Statement};
use claw_orm::{field, Model, Row, Value};

#[derive(Debug)]
struct Item { id: i64 }
impl Model for Item {
    const TABLE: &'static str = "items";
    fn from_row(row: &Row) -> claw_orm::Result<Self> { Ok(Self { id: field(row, "id")? }) }
    fn key(&self) -> Value { self.id.into() }
}

fn database() -> Database {
    Database::new(|| {
        let mut c = SqliteConnection::in_memory()?;
        c.execute(&Statement::new("CREATE TABLE items (id INTEGER PRIMARY KEY)"))?;
        for id in 1_i64..=5 { c.execute(&Statement::new("INSERT INTO items (id) VALUES (?)").bind(id))?; }
        Ok(Box::new(c))
    })
}

#[test]
fn pagination_metadata_and_boundaries_are_stable() {
    let scope = DatabaseScope::new(database());
    scope.run(|| {
        let page = Item::query().paginate(2).unwrap();
        assert_eq!(page.page(), 1);
        assert_eq!(page.per_page(), 2);
        assert_eq!(page.total(), 5);
        assert_eq!(page.last_page(), 3);
        assert!(!page.has_previous());
        assert!(page.has_next());
        assert_eq!(page.items().len(), 2);
    });
}

#[test]
fn pagination_rejects_invalid_size_and_offset_overflow() {
    let scope = DatabaseScope::new(database());
    scope.run(|| {
        let error = Item::query().paginate(0).unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::InvalidInput);
    });

    let scope = DatabaseScope::optional(Some(database()), Ok(u64::MAX));
    scope.run(|| {
        let error = Item::query().paginate(1000).unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::InvalidInput);
    });
}
