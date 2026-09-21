mod common;
use claw_orm::{
    with_scoped_connection, Connection, Direction, ErrorKind, Model, ModelQuery, Statement, Value,
};
use common::{setup, Post, Role, User};

#[test]
fn all_four_relationships_return_normal_lazy_model_queries() {
    let mut c = setup();
    let user = User {
        id: 1,
        name: "Ada".into(),
    };
    let posts: ModelQuery<Post> = User::posts()
        .query_for(&user)
        .unwrap()
        .where_("published", true)
        .order_by("id", Direction::Desc);
    assert!(c.statements.is_empty());
    assert_eq!(posts.get_on(&mut c).unwrap().len(), 1);
    assert_eq!(
        User::profile()
            .query_for(&user)
            .unwrap()
            .first_on(&mut c)
            .unwrap()
            .unwrap()
            .user_id,
        1
    );
    let post = Post::find_on(&mut c, 1).unwrap().unwrap();
    assert_eq!(
        Post::user()
            .query_for(&post)
            .unwrap()
            .first_on(&mut c)
            .unwrap(),
        Some(user)
    );
    let user = User {
        id: 1,
        name: "Ada".into(),
    };
    let roles: ModelQuery<Role> = User::roles().query_for(&user).unwrap();
    assert_eq!(
        roles.where_("roles.id", 10).get_on(&mut c).unwrap()[0].name,
        "Editor"
    );
}

#[test]
fn relationship_scope_survives_or_filters_for_reads_count_and_mutations() {
    let mut c = setup();
    let user = User {
        id: 1,
        name: "Ada".into(),
    };
    let query = User::posts()
        .query_for(&user)
        .unwrap()
        .where_("id", 1)
        .or_where("id", 3);
    assert_eq!(query.count_on(&mut c).unwrap(), 1);
    assert_eq!(query.get_on(&mut c).unwrap()[0].id, 1);
    assert_eq!(
        query
            .update_on(&mut c, [("title", "Updated")])
            .unwrap()
            .affected_rows,
        1
    );
    let deleted = User::posts()
        .query_for(&user)
        .unwrap()
        .or_where("id", 3)
        .delete_on(&mut c)
        .unwrap();
    assert_eq!(deleted.affected_rows, 0);
    assert!(Post::find_on(&mut c, 3).unwrap().is_some());
}

#[test]
fn absent_belongs_to_stays_empty_even_with_or_filters() {
    let mut c = setup();
    let post = Post::find_on(&mut c, 4).unwrap().unwrap();
    let query = Post::user().query_for(&post).unwrap().or_where("id", 1);
    assert_eq!(query.count_on(&mut c).unwrap(), 0);
    assert!(query.get_on(&mut c).unwrap().is_empty());
}

#[test]
fn many_to_many_query_preserves_duplicates_and_paginates_join_rows() {
    let mut c = setup();
    User::roles()
        .attach_on(
            &mut c,
            &User {
                id: 1,
                name: "Ada".into(),
            },
            10,
        )
        .unwrap();
    let query = User::roles()
        .query_for(&User {
            id: 1,
            name: "Ada".into(),
        })
        .unwrap();
    assert_eq!(query.count_on(&mut c).unwrap(), 3);
    let page = query
        .order_by("roles.id", Direction::Asc)
        .paginate_on(&mut c, 1, 2)
        .unwrap();
    assert_eq!(page.total(), 3);
    assert_eq!(
        page.items().iter().map(|role| role.id).collect::<Vec<_>>(),
        [10, 10]
    );
}

#[test]
fn scoped_queries_execute_on_the_scoped_connection() {
    let mut c = setup();
    with_scoped_connection(&mut c, || {
        let user = User::find(1).unwrap().unwrap();
        assert_eq!(
            User::posts().query_for(&user).unwrap().get().unwrap().len(),
            2
        );
        assert_eq!(
            User::roles()
                .query_for(&user)
                .unwrap()
                .paginate(1)
                .unwrap()
                .total(),
            2
        );
    });
}

#[test]
fn joined_mutations_fail_before_execution_and_parent_keys_are_bound() {
    let mut c = setup();
    let relation = User::roles();
    let user = User {
        id: 1,
        name: "Ada".into(),
    };
    let query = relation.query_for(&user).unwrap();
    assert_eq!(
        query
            .to_statement(claw_orm::Driver::Postgres)
            .unwrap()
            .bindings(),
        [Value::U64(1)]
    );
    assert_eq!(
        query.delete_on(&mut c).unwrap_err().kind(),
        &ErrorKind::Query
    );
    assert!(c.statements.is_empty());
}

#[test]
fn has_one_query_can_inspect_invalid_data_while_loader_enforces_cardinality() {
    let mut c = setup();
    c.execute(&Statement::new("INSERT INTO profiles VALUES (3, 1)"))
        .unwrap();
    let user = User {
        id: 1,
        name: "Ada".into(),
    };
    assert_eq!(
        User::profile()
            .query_for(&user)
            .unwrap()
            .count_on(&mut c)
            .unwrap(),
        2
    );
    assert_eq!(
        User::profile().load_on(&mut c, &[user]).unwrap_err().kind(),
        &ErrorKind::Decode
    );
}
