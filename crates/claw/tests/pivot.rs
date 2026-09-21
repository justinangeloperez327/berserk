use berserk_database::{drivers::sqlite::SqliteConnection, Connection, Query, Statement};
use claw_orm::{
    field, with_scoped_connection, BelongsToMany, ErrorKind, Model, Result, Row, SyncResult,
    Transaction, Value,
};

// Deliberately not Clone: neither side of a relationship needs that bound.
#[derive(Debug)]
struct User(u64);
#[derive(Debug)]
struct Role(u64);
impl Model for User {
    const TABLE: &'static str = "users";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self(field(row, "id")?))
    }
    fn key(&self) -> Value {
        self.0.into()
    }
}
impl Model for Role {
    const TABLE: &'static str = "roles";
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self(field(row, "id")?))
    }
    fn key(&self) -> Value {
        self.0.into()
    }
}
fn roles() -> BelongsToMany<User, Role> {
    BelongsToMany::new("role_user", "user_id", "role_id", User::key, Role::key)
}
fn setup(unique: bool) -> SqliteConnection {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.connection_mut()
        .execute_batch(&format!(
            "PRAGMA foreign_keys = ON;
         CREATE TABLE users (id INTEGER PRIMARY KEY);
         CREATE TABLE roles (id INTEGER PRIMARY KEY);
         CREATE TABLE role_user (
             user_id INTEGER NOT NULL REFERENCES users(id),
             role_id INTEGER NOT NULL REFERENCES roles(id){}
         );
         INSERT INTO users VALUES (1), (2);
         INSERT INTO roles VALUES (10), (11), (12), (13);",
            if unique {
                ", UNIQUE(user_id, role_id)"
            } else {
                ""
            }
        ))
        .unwrap();
    c
}
fn ids(c: &mut dyn Connection, user: u64) -> Vec<u64> {
    Query::table("role_user")
        .where_("user_id", "=", user)
        .order_by("role_id", claw_orm::Direction::Asc)
        .get(c)
        .unwrap()
        .iter()
        .map(|row| field(row, "role_id").unwrap())
        .collect()
}

#[test]
fn attach_one_and_many_bind_keys_and_deduplicate_requested_ids() {
    let mut c = setup(true);
    assert_eq!(
        roles()
            .attach_on(&mut c, &User(1), 10)
            .unwrap()
            .affected_rows,
        1
    );
    assert_eq!(
        roles()
            .attach_many_on(&mut c, &User(1), [11, 12, 11])
            .unwrap(),
        2
    );
    assert_eq!(ids(&mut c, 1), [10, 11, 12]);
    assert!(ids(&mut c, 2).is_empty());
    assert_eq!(
        roles()
            .attach_many_on(&mut c, &User(1), [Value::I64(13), Value::U64(13)])
            .unwrap(),
        1
    );
}

#[test]
fn detach_one_many_and_all_preserve_shared_relationships() {
    let mut c = setup(true);
    roles()
        .attach_many_on(&mut c, &User(1), [10, 11, 12])
        .unwrap();
    roles().attach_on(&mut c, &User(2), 10).unwrap();
    assert_eq!(
        roles()
            .detach_on(&mut c, &User(1), 10)
            .unwrap()
            .affected_rows,
        1
    );
    assert_eq!(
        roles()
            .detach_many_on(&mut c, &User(1), [11, 11])
            .unwrap()
            .affected_rows,
        1
    );
    assert_eq!(ids(&mut c, 1), [12]);
    assert_eq!(
        roles()
            .detach_all_on(&mut c, &User(1))
            .unwrap()
            .affected_rows,
        1
    );
    assert!(ids(&mut c, 1).is_empty());
    assert_eq!(ids(&mut c, 2), [10]);
}

#[test]
fn empty_attach_and_detach_are_noops() {
    let mut c = setup(true);
    roles().attach_on(&mut c, &User(1), 10).unwrap();
    assert_eq!(
        roles()
            .attach_many_on(&mut c, &User(1), [] as [u64; 0])
            .unwrap(),
        0
    );
    assert_eq!(
        roles()
            .detach_many_on(&mut c, &User(1), [] as [u64; 0])
            .unwrap()
            .affected_rows,
        0
    );
    assert_eq!(ids(&mut c, 1), [10]);
}

#[test]
fn sync_identical_add_remove_replace_and_empty_are_atomic_membership_changes() {
    let mut c = setup(true);
    let relation = roles();
    let user = User(1);
    relation.attach_many_on(&mut c, &user, [10, 11]).unwrap();
    relation.attach_on(&mut c, &User(2), 10).unwrap();
    let cases: &[(Vec<u64>, Vec<u64>, Vec<u64>)] = &[
        (vec![10, 11, 11], vec![], vec![]),
        (vec![10, 11, 12], vec![12], vec![]),
        (vec![10, 12], vec![], vec![11]),
        (vec![11, 12, 13], vec![11, 13], vec![10]),
        (vec![], vec![], vec![11, 12, 13]),
    ];
    for (desired, attached, detached) in cases {
        let result = relation
            .sync_on(&mut c, &user, desired.iter().copied())
            .unwrap();
        assert_eq!(
            result.attached,
            attached
                .iter()
                .copied()
                .map(Value::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            result.detached,
            detached
                .iter()
                .copied()
                .map(Value::from)
                .collect::<Vec<_>>()
        );
        let mut expected = desired.clone();
        expected.sort_unstable();
        expected.dedup();
        assert_eq!(ids(&mut c, 1), expected);
        assert_eq!(ids(&mut c, 2), [10]);
    }
}

#[test]
fn null_and_invalid_keys_fail_before_writes_including_batch_tail() {
    let mut c = setup(true);
    for key in [
        Value::Null,
        Value::Bool(true),
        Value::F64(f64::NAN),
        Value::F64(1.5),
        Value::Text(String::new()),
        Value::Bytes(vec![]),
    ] {
        assert_eq!(
            roles()
                .attach_on(&mut c, &User(1), key.clone())
                .unwrap_err()
                .kind(),
            &ErrorKind::InvalidInput
        );
        assert_eq!(
            roles()
                .detach_on(&mut c, &User(1), key.clone())
                .unwrap_err()
                .kind(),
            &ErrorKind::InvalidInput
        );
        assert!(roles()
            .attach_many_on(&mut c, &User(1), [Value::from(10), key.clone()])
            .is_err());
        assert!(roles()
            .sync_on(&mut c, &User(1), [Value::from(10), key])
            .is_err());
        assert!(ids(&mut c, 1).is_empty());
    }
    let null_parent = BelongsToMany::<User, Role>::new(
        "role_user",
        "user_id",
        "role_id",
        |_| Value::Null,
        Role::key,
    );
    assert!(null_parent.attach_on(&mut c, &User(1), 10).is_err());
    assert!(null_parent.detach_all_on(&mut c, &User(1)).is_err());
    assert!(null_parent.sync_on(&mut c, &User(1), [10]).is_err());
}

#[test]
fn constraints_propagate_and_batch_and_sync_roll_back_partial_changes() {
    let mut c = setup(true);
    roles().attach_on(&mut c, &User(1), 10).unwrap();
    assert_eq!(
        roles().attach_on(&mut c, &User(1), 10).unwrap_err().kind(),
        &ErrorKind::Constraint
    );
    assert_eq!(
        roles().attach_on(&mut c, &User(1), 999).unwrap_err().kind(),
        &ErrorKind::Constraint
    );
    assert!(roles().attach_many_on(&mut c, &User(1), [11, 999]).is_err());
    assert_eq!(ids(&mut c, 1), [10]);
    // Delete 10 and insert 11, then fail inserting 999. Both changes must roll back.
    assert_eq!(
        roles()
            .sync_on(&mut c, &User(1), [11, 999])
            .unwrap_err()
            .kind(),
        &ErrorKind::Constraint
    );
    assert_eq!(ids(&mut c, 1), [10]);
}

#[test]
fn duplicate_rows_are_retained_by_sync_and_removed_together_by_detach() {
    let mut c = setup(false);
    roles().attach_on(&mut c, &User(1), 10).unwrap();
    roles().attach_on(&mut c, &User(1), 10).unwrap();
    assert_eq!(
        roles().sync_on(&mut c, &User(1), [10, 10]).unwrap(),
        SyncResult::default()
    );
    assert_eq!(ids(&mut c, 1), [10, 10]);
    assert_eq!(
        roles()
            .detach_on(&mut c, &User(1), 10)
            .unwrap()
            .affected_rows,
        2
    );
}

#[test]
fn scoped_variants_share_the_explicit_connection() {
    let mut c = setup(true);
    with_scoped_connection(&mut c, || -> Result<()> {
        let relation = roles();
        relation.attach(&User(1), 10)?;
        relation.attach_many(&User(1), [11, 12])?;
        relation.detach(&User(1), 10)?;
        relation.detach_many(&User(1), [11])?;
        assert_eq!(relation.sync(&User(1), [10])?.attached, [Value::U64(10)]);
        relation.detach_all(&User(1))?;
        Ok(())
    })
    .unwrap();
    assert!(ids(&mut c, 1).is_empty());
}

#[test]
fn nested_sync_fails_before_changes_and_single_writes_join_the_outer_transaction() {
    let mut c = setup(true);
    with_scoped_connection(&mut c, || {
        let result: Result<()> = Transaction::run(|| {
            roles().attach(&User(1), 10)?;
            roles().sync(&User(1), [11])?;
            Ok(())
        });
        assert_eq!(result.unwrap_err().kind(), &ErrorKind::Transaction);
    });
    assert!(ids(&mut c, 1).is_empty());
}

#[test]
fn custom_table_and_columns_and_text_keys_remain_bound() {
    let mut c = SqliteConnection::in_memory().unwrap();
    c.execute(&Statement::new(
        "CREATE TABLE memberships (account TEXT, permission TEXT)",
    ))
    .unwrap();
    let relation = BelongsToMany::<String, Role>::new(
        "memberships",
        "account",
        "permission",
        |p| p.clone().into(),
        Role::key,
    );
    let parent = "account' OR 1=1 --".to_owned();
    let related = "role'); DROP TABLE memberships; --";
    relation.attach_on(&mut c, &parent, related).unwrap();
    relation
        .attach_on(&mut c, &"other".to_owned(), related)
        .unwrap();
    assert_eq!(
        relation.sync_on(&mut c, &parent, [related]).unwrap(),
        SyncResult::default()
    );
    assert_eq!(
        relation
            .detach_on(&mut c, &parent, related)
            .unwrap()
            .affected_rows,
        1
    );
    assert_eq!(Query::table("memberships").count(&mut c).unwrap(), 1);
}
