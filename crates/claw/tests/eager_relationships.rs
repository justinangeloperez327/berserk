mod common;
use claw_orm::{
    with_scoped_connection, Connection, Direction, ErrorKind, Model, Relationship, Statement, Value,
};
use common::{setup, Post, User};

#[test]
fn named_eager_loading_preserves_query_composition_after_with() {
    let statement = User::query()
        .with(["roles"])
        .where_("name", "Ada")
        .where_in("id", [1_u64, 2])
        .where_not_null("name")
        .order_by("id", Direction::Desc)
        .limit(5)
        .offset(1)
        .to_statement(Driver::Sqlite)
        .unwrap();

    assert!(statement.sql().contains("WHERE"));
    assert!(statement.sql().contains("ORDER BY"));
    assert!(statement.sql().contains("LIMIT"));
    assert!(statement.sql().contains("OFFSET"));
    assert_eq!(
        statement.bindings(),
        [Value::from("Ada"), Value::U64(1), Value::U64(2)]
    );
}

#[test]
fn model_query_eager_loads_shared_roles_in_exactly_three_queries() {
    let mut c = setup();
    let loaded = User::query().with(User::roles()).get_on(&mut c).unwrap();
    assert_eq!(loaded.models.len(), 3);
    assert_eq!(loaded.relations.get(&Value::U64(1)).unwrap().len(), 2);
    assert_eq!(
        loaded.relations.get(&Value::U64(2)).unwrap()[0].name,
        "Editor"
    );
    assert!(loaded.relations.get(&Value::U64(3)).is_none());
    assert_eq!(c.statements.len(), 3);
    assert!(c.statements[0].sql().contains("FROM \"users\""));
    assert!(c.statements[1].sql().contains("FROM \"role_user\""));
    assert!(c.statements[2].sql().contains("FROM \"roles\""));
    assert_eq!(
        c.statements[1].bindings(),
        [Value::U64(1), Value::U64(2), Value::U64(3)]
    );
}

#[test]
fn eager_pagination_loads_only_the_page_and_adds_one_count_query() {
    let mut c = setup();
    let loaded = User::query()
        .order_by("id", Direction::Asc)
        .with(User::roles())
        .paginate_on(&mut c, 2, 1)
        .unwrap();
    assert_eq!(loaded.page.total(), 3);
    assert_eq!(loaded.page.items()[0].id, 2);
    assert_eq!(loaded.relations.get(&Value::U64(2)).unwrap().len(), 1);
    assert!(loaded.relations.get(&Value::U64(1)).is_none());
    assert_eq!(c.statements.len(), 4);
    assert_eq!(c.statements[2].bindings(), [Value::U64(2)]);
}

#[test]
fn multiple_eager_relations_batch_each_relation_without_parent_queries() {
    let mut c = setup();
    let loaded = User::query()
        .with(User::posts())
        .with(User::roles())
        .with(User::profile())
        .get_on(&mut c)
        .unwrap();
    let ((posts, roles), profiles) = loaded.relations;
    assert_eq!(posts.get(&Value::U64(1)).unwrap().len(), 2);
    assert_eq!(roles.get(&Value::U64(1)).unwrap().len(), 2);
    assert_eq!(profiles.get(&Value::U64(1)).unwrap().len(), 1);
    assert_eq!(c.statements.len(), 5);
}

#[test]
fn empty_parents_and_empty_pivots_skip_unnecessary_queries() {
    let mut c = setup();
    let loaded = User::where_("id", 999)
        .with(User::roles())
        .get_on(&mut c)
        .unwrap();
    assert!(loaded.models.is_empty());
    assert_eq!(c.statements.len(), 1);
    c.statements.clear();
    let loaded = User::where_("id", 3)
        .with(User::roles())
        .get_on(&mut c)
        .unwrap();
    assert!(loaded.relations.is_empty());
    assert_eq!(c.statements.len(), 2);
}

#[test]
fn eager_loading_preserves_duplicate_pivot_rows_without_clone_models() {
    let mut c = setup();
    c.execute(&Statement::new(
        "INSERT INTO role_user (user_id, role_id) VALUES (1, 10)",
    ))
    .unwrap();
    c.statements.clear();
    let loaded = User::query().with(User::roles()).get_on(&mut c).unwrap();
    assert_eq!(loaded.relations.get(&Value::U64(1)).unwrap().len(), 3);
    assert_eq!(loaded.relations.get(&Value::U64(2)).unwrap().len(), 1);
    assert_eq!(c.statements.len(), 3);
}

#[test]
fn scoped_eager_loading_pagination_and_direct_loaders_use_the_same_connection() {
    let mut c = setup();
    with_scoped_connection(&mut c, || {
        let loaded = User::query().with(User::roles()).paginate(2).unwrap();
        assert_eq!(loaded.page.items().len(), 2);
        let users = User::all().unwrap();
        assert_eq!(User::roles().load(&users).unwrap().len(), 2);
        assert_eq!(Relationship::load(&User::posts(), &users).unwrap().len(), 2);
        assert_eq!(
            Relationship::load(&User::profile(), &users).unwrap().len(),
            2
        );
        assert_eq!(
            (User::posts(), User::roles()).load(&users).unwrap().0.len(),
            2
        );
        let posts = Post::all().unwrap();
        assert_eq!(Relationship::load(&Post::user(), &posts).unwrap().len(), 2);
    });
}

#[test]
fn has_one_and_related_decode_and_query_errors_propagate_through_eager_queries() {
    let mut c = setup();
    c.execute(&Statement::new("INSERT INTO profiles VALUES (3, 1)"))
        .unwrap();
    let result = User::query().with(User::profile()).get_on(&mut c);
    assert_eq!(result.err().unwrap().kind(), &ErrorKind::Decode);
    c.execute(&Statement::new(
        "UPDATE roles SET name = NULL WHERE id = 10",
    ))
    .unwrap();
    assert_eq!(
        User::query()
            .with(User::roles())
            .get_on(&mut c)
            .err()
            .unwrap()
            .kind(),
        &ErrorKind::Decode
    );
    c.execute(&Statement::new("DROP TABLE role_user")).unwrap();
    assert!(User::query().with(User::roles()).get_on(&mut c).is_err());
}

#[test]
#[allow(deprecated)]
fn stable_one_x_explicit_load_signatures_remain_usable() {
    let mut c = setup();
    let users = User::all_on(&mut c).unwrap();
    assert_eq!(User::posts().load(&mut c, &users).unwrap().len(), 2);
    assert_eq!(User::profile().load(&mut c, &users).unwrap().len(), 2);
    let posts = Post::all_on(&mut c).unwrap();
    assert_eq!(Post::user().load(&mut c, &posts).unwrap().len(), 2);
}
