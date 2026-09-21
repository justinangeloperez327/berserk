#![allow(dead_code)]
use berserk_database::{
    drivers::sqlite::SqliteConnection, Capabilities, Execution, Transaction, TransactionOptions,
};
use claw_orm::{
    field, BelongsTo, BelongsToMany, Connection, Driver, HasMany, HasOne, Model, Result, Row,
    Statement, Value,
};

#[derive(Debug, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
}
#[derive(Debug, PartialEq)]
pub struct Post {
    pub id: u64,
    pub user_id: Option<u64>,
    pub title: String,
    pub published: bool,
}
#[derive(Debug, PartialEq)]
pub struct Profile {
    pub id: u64,
    pub user_id: u64,
}
#[derive(Debug, PartialEq)]
pub struct Role {
    pub id: u64,
    pub name: String,
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
            title: field(row, "title")?,
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
    pub fn posts() -> HasMany<Self, Post> {
        HasMany::new("user_id", Self::key, |post: &Post| {
            post.user_id.map(Value::from).unwrap_or(Value::Null)
        })
    }
    pub fn profile() -> HasOne<Self, Profile> {
        HasOne::new("user_id", Self::key, |profile: &Profile| {
            profile.user_id.into()
        })
    }
    pub fn roles() -> BelongsToMany<Self, Role> {
        BelongsToMany::new("role_user", "user_id", "role_id", Self::key, Role::key)
    }
}
impl Post {
    pub fn user() -> BelongsTo<Self, User> {
        BelongsTo::new("id", |post: &Post| post.user_id.map(Value::from), User::key)
    }
}

pub fn setup() -> Counting {
    let mut connection = SqliteConnection::in_memory().unwrap();
    connection.connection_mut().execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
         CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), title TEXT, published BOOLEAN);
         CREATE TABLE profiles (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id));
         CREATE TABLE roles (id INTEGER PRIMARY KEY, name TEXT);
         CREATE TABLE role_user (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), role_id INTEGER REFERENCES roles(id));
         INSERT INTO users VALUES (1, 'Ada'), (2, 'Lin'), (3, 'Grace');
         INSERT INTO posts VALUES (1, 1, 'First', 1), (2, 1, 'Draft', 0), (3, 2, 'Other', 1), (4, NULL, 'Unassigned', 0);
         INSERT INTO profiles VALUES (1, 1), (2, 2);
         INSERT INTO roles VALUES (10, 'Editor'), (11, 'Admin');
         INSERT INTO role_user (user_id, role_id) VALUES (1, 10), (1, 11), (2, 10);"
    ).unwrap();
    Counting {
        connection,
        statements: vec![],
    }
}

pub struct Counting {
    pub connection: SqliteConnection,
    pub statements: Vec<Statement>,
}
impl Connection for Counting {
    fn driver(&self) -> Driver {
        self.connection.driver()
    }
    fn capabilities(&self) -> Capabilities {
        self.connection.capabilities()
    }
    fn query(&mut self, statement: &Statement) -> Result<Vec<Row>> {
        self.statements.push(statement.clone());
        self.connection.query(statement)
    }
    fn execute(&mut self, statement: &Statement) -> Result<Execution> {
        self.statements.push(statement.clone());
        self.connection.execute(statement)
    }
    fn begin(&mut self, options: TransactionOptions) -> Result<Box<dyn Transaction + '_>> {
        self.connection.begin(options)
    }
    fn ping(&mut self) -> Result<()> {
        self.connection.ping()
    }
}
