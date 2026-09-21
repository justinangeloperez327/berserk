//! Run: cargo run -p claw-orm --example relationships --features sqlite
use berserk_database::{drivers::sqlite::SqliteConnection, Query};
use claw_orm::prelude::*;
use claw_orm::{with_scoped_connection, Direction, Result, Row, Value};

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
}
#[derive(Debug)]
struct Post {
    id: u64,
    user_id: u64,
    published: bool,
}
#[derive(Debug)]
struct Profile {
    id: u64,
    user_id: u64,
}
#[derive(Debug)]
struct Role {
    id: u64,
    name: String,
}

impl Model for User {
    const TABLE: &'static str = "users";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
}
impl Model for Post {
    const TABLE: &'static str = "posts";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            user_id: field(row, "user_id")?,
            published: field(row, "published")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
}
impl Model for Profile {
    const TABLE: &'static str = "profiles";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            user_id: field(row, "user_id")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
}
impl Model for Role {
    const TABLE: &'static str = "roles";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
}
impl User {
    fn posts() -> HasMany<Self, Post> {
        HasMany::new("user_id", Self::key, |post: &Post| post.user_id.into())
    }
    fn profile() -> HasOne<Self, Profile> {
        HasOne::new("user_id", Self::key, |profile: &Profile| {
            profile.user_id.into()
        })
    }
    fn roles() -> BelongsToMany<Self, Role> {
        BelongsToMany::new("role_user", "user_id", "role_id", Self::key, Role::key)
    }
}
impl Post {
    fn user() -> BelongsTo<Self, User> {
        BelongsTo::new("id", |post: &Post| Some(post.user_id.into()), User::key)
    }
}

fn main() -> Result<()> {
    let mut connection = SqliteConnection::in_memory()?;
    // Schema is trusted static SQL. Application values below are bound separately.
    for sql in [
        "PRAGMA foreign_keys = ON",
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id), published BOOLEAN NOT NULL)",
        "CREATE TABLE profiles (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL UNIQUE REFERENCES users(id))",
        "CREATE TABLE roles (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE role_user (user_id INTEGER NOT NULL REFERENCES users(id), role_id INTEGER NOT NULL REFERENCES roles(id), UNIQUE(user_id, role_id))",
    ] { Query::raw(sql).execute(&mut connection)?; }
    for (id, name) in [(1, "Ada"), (2, "Lin")] {
        User::create_on(
            &mut connection,
            [("id", id.into()), ("name", Value::from(name))],
        )?;
    }
    for (id, name) in [(10, "Editor"), (11, "Admin"), (12, "Reviewer")] {
        Role::create_on(
            &mut connection,
            [("id", id.into()), ("name", Value::from(name))],
        )?;
    }
    Post::create_on(
        &mut connection,
        [
            ("id", 1.into()),
            ("user_id", 1.into()),
            ("published", Value::from(true)),
        ],
    )?;
    Profile::create_on(&mut connection, [("id", 1), ("user_id", 1)])?;

    with_scoped_connection(&mut connection, || -> Result<()> {
        let ada = User::find(1)?.expect("seeded user");
        let lin = User::find(2)?.expect("seeded user");
        let roles = User::roles();
        roles.attach(&ada, 10)?;
        roles.attach_many(&ada, [11, 12])?;
        roles.attach(&lin, 10)?;
        roles.detach(&ada, 12)?;
        roles.detach_many(&ada, [11])?;
        let changes = roles.sync(&ada, [10, 12, 12])?;
        assert_eq!(changes.attached, [Value::U64(12)]);
        assert!(changes.detached.is_empty());

        let posts = User::posts()
            .query_for(&ada)?
            .where_("published", true)
            .get()?;
        assert!(posts[0].published);
        assert_eq!(
            Post::user().query_for(&posts[0])?.first()?.unwrap().name,
            "Ada"
        );
        assert!(User::profile().query_for(&ada)?.first()?.is_some());
        let names: Vec<_> = roles
            .query_for(&ada)?
            .order_by("roles.name", Direction::Asc)
            .get()?
            .into_iter()
            .map(|role| role.name)
            .collect();
        assert_eq!(names, ["Editor", "Reviewer"]);

        let loaded = User::query()
            .with(User::posts())
            .with(User::roles())
            .get()?;
        let (posts, roles_by_user) = loaded.relations;
        assert_eq!(posts.get(&ada.key()).unwrap().len(), 1);
        assert_eq!(roles_by_user.get(&ada.key()).unwrap().len(), 2);
        assert_eq!(roles_by_user.get(&lin.key()).unwrap().len(), 1);
        let page = User::query()
            .order_by("id", Direction::Asc)
            .with(User::profile())
            .paginate(1)?;
        assert_eq!(page.page.total(), 2);
        assert_eq!(page.relations.get(&ada.key()).unwrap().len(), 1);
        roles.detach_all(&ada)?;
        assert_eq!(roles.query_for(&lin)?.count()?, 1);
        println!("All four relationships, eager pagination, and atomic pivot changes verified.");
        Ok(())
    })
}
