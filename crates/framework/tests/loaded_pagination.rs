#![cfg(all(feature = "claw", feature = "sqlite", feature = "view"))]

use berserk::{
    axe::Value as AxeValue,
    claw::{DatabaseScope, Direction},
    database::{drivers::sqlite::SqliteConnection, Connection, Database, Statement},
    response, Json, Model, Result,
};

#[derive(Model)]
#[table("users")]
#[has_many(Post, "posts", foreign_key = "user_id")]
struct User {
    #[primary_key]
    id: i64,

    #[fillable]
    name: String,
}

#[derive(Model)]
#[table("posts")]
struct Post {
    #[primary_key]
    id: i64,

    #[fillable]
    user_id: i64,

    #[fillable]
    title: String,
}

fn database() -> Database {
    Database::new(|| {
        let mut connection = SqliteConnection::in_memory()?;
        connection.execute(&Statement::new(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        ))?;
        connection.execute(&Statement::new(
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL)",
        ))?;
        connection.execute(&Statement::new(
            "INSERT INTO users (id, name) VALUES (1, 'Ada'), (2, 'Grace')",
        ))?;
        connection.execute(&Statement::new(
            "INSERT INTO posts (id, user_id, title) VALUES (10, 1, 'First'), (11, 1, 'Second'), (12, 2, 'Third')",
        ))?;
        Ok(connection)
    })
}

#[test]
fn named_loaded_page_is_first_class_json_and_axe_data() -> Result<()> {
    DatabaseScope::new(database()).run(|| {
        let loaded = User::query()
            .with(["posts"])
            .order_by("id", Direction::Asc)
            .paginate(1)?;

        assert_eq!(loaded.current_page(), 1);
        assert_eq!(loaded.per_page(), 1);
        assert_eq!(loaded.total(), 2);
        assert_eq!(loaded.last_page(), 2);
        assert!(!loaded.has_previous());
        assert!(loaded.has_next());
        assert_eq!(loaded.items().len(), 1);
        assert_eq!(loaded.items()[0].name, "Ada");

        let response = response().json(&loaded)?;
        let Json::Object(body) = Json::parse(response.body()).expect("valid loaded-page JSON") else {
            panic!("loaded page should serialize as an object");
        };
        let Json::Array(data) = body.get("data").expect("data") else {
            panic!("loaded page data should be an array");
        };
        assert_eq!(data.len(), 1);
        let Json::Object(user) = &data[0] else {
            panic!("loaded model should serialize as an object");
        };
        assert_eq!(user.get("name").and_then(Json::as_str), Some("Ada"));
        let Json::Array(posts) = user.get("posts").expect("posts") else {
            panic!("has-many relationship should serialize as an array");
        };
        assert_eq!(posts.len(), 2);
        let Json::Object(meta) = body.get("meta").expect("meta") else {
            panic!("loaded page metadata should be an object");
        };
        assert_eq!(meta.get("current_page"), Some(&Json::from(1_u64)));
        assert_eq!(meta.get("per_page"), Some(&Json::from(1_u64)));
        assert_eq!(meta.get("total"), Some(&Json::from(2_u64)));
        assert_eq!(meta.get("last_page"), Some(&Json::from(2_u64)));

        let AxeValue::Object(view) = berserk::views::value(&loaded) else {
            panic!("loaded page should become an Axe object");
        };
        let AxeValue::List(view_data) = view.get("data").expect("view data") else {
            panic!("Axe data should be a list");
        };
        assert_eq!(view_data.len(), 1);
        let AxeValue::Object(view_meta) = view.get("meta").expect("view meta") else {
            panic!("Axe metadata should be an object");
        };
        assert_eq!(
            view_meta.get("total"),
            Some(&AxeValue::Number("2".to_owned()))
        );

        Ok(())
    })
}
