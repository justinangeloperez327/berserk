#![cfg(feature = "sqlite")]

use berserk_database::{
    drivers::sqlite::SqliteConnection, scope::with_scoped_connection, Connection, Direction,
    Statement, Value,
};
use claw_orm::{field, IntoInsert, IntoUpdate, Model, Result, Row};

#[derive(Debug, PartialEq)]
struct User {
    id: i64,
    name: String,
    email: Option<String>,
    score: i64,
}
impl Model for User {
    const TABLE: &'static str = "users";
    const FILLABLE: &'static [&'static str] = &["name", "email", "score"];
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            email: field(row, "email")?,
            score: field(row, "score")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
}
struct NewUser(&'static str, Option<&'static str>, i64);
impl IntoInsert<User> for NewUser {
    fn into_insert(self) -> Result<Vec<(String, Value)>> {
        Ok(vec![
            ("name".into(), self.0.into()),
            (
                "email".into(),
                self.1.map(Value::from).unwrap_or(Value::Null),
            ),
            ("score".into(), self.2.into()),
        ])
    }
}
struct Rename(&'static str);
impl IntoUpdate<User> for Rename {
    fn into_update(self) -> Result<Vec<(String, Value)>> {
        Ok(vec![("name".into(), self.0.into())])
    }
}

fn setup() -> SqliteConnection {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.execute(&Statement::new("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, email TEXT NULL, score INTEGER NOT NULL)")).unwrap();
    c
}

#[test]
fn terminal_operations_share_one_predictable_query_contract() {
    let mut c = setup();
    with_scoped_connection(&mut c, || {
        let ada = User::create(NewUser("Ada", Some("ada@example.com"), 95)).unwrap();
        User::create(NewUser("Grace", None, 88)).unwrap();
        User::create(NewUser("Linus", Some("linus@example.com"), 70)).unwrap();

        assert_eq!(User::count().unwrap(), 3);
        assert!(User::where_("name", "Ada").exists().unwrap());
        assert_eq!(User::find(ada.id).unwrap().unwrap().name, "Ada");
        assert!(User::find(999_i64).unwrap().is_none());
        assert!(User::find_or_fail(999_i64).is_err());

        let users = User::where_between("score", 80_i64, 100_i64)
            .order_by("score", Direction::Desc)
            .get()
            .unwrap();
        assert_eq!(users.len(), 2);

        let first = User::where_not_null("email")
            .order_by("score", Direction::Desc)
            .first()
            .unwrap()
            .unwrap();
        assert_eq!(first.name, "Ada");
    });
}

#[test]
fn create_update_fresh_refresh_delete_and_destroy_have_consistent_lifecycle() {
    let mut c = setup();
    with_scoped_connection(&mut c, || {
        let mut user = User::create(NewUser("Ada", Some("ada@example.com"), 95)).unwrap();
        let original_id = user.id;

        user.update(Rename("Augusta")).unwrap();
        assert_eq!(user.name, "Augusta");
        assert_eq!(user.id, original_id);

        external_update_for_scope(original_id, "Ada Lovelace").unwrap();
        let fresh = user.fresh().unwrap().unwrap();
        assert_eq!(fresh.name, "Ada Lovelace");
        assert_eq!(user.name, "Augusta");
        assert!(user.refresh().unwrap());
        assert_eq!(user.name, "Ada Lovelace");

        assert_eq!(user.delete().unwrap().affected_rows, 1);
        assert!(user.fresh().unwrap().is_none());

        let second = User::create(NewUser("Grace", None, 88)).unwrap();
        assert_eq!(User::destroy(second.id).unwrap().affected_rows, 1);
        assert!(!User::where_("id", second.id).exists().unwrap());
    });
}

fn external_update_for_scope(id: i64, name: &str) -> Result<()> {
    berserk_database::scope::with_connection(|connection| {
        connection.execute(
            &Statement::new("UPDATE users SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id),
        )?;
        Ok(())
    })
}
