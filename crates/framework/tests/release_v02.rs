#![cfg(all(feature = "claw", feature = "sqlite"))]
use berserk::{
    claw::{field, DatabaseScope, HasMany, IntoInsert, IntoUpdate, Model, Row, Transaction, Value},
    database::{drivers::sqlite::SqliteConnection, Connection, Database, Statement},
    ApiResource, App, Error, FormRequest, FromJson, Headers, IntoResponse, Json, Method, Request,
    ResourceCollection, Response, Result, ValidationErrors, ValidationResult,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug)]
struct User {
    id: i64,
    name: String,
    parent_id: Option<i64>,
}
impl Model for User {
    const TABLE: &'static str = "users";
    const FILLABLE: &'static [&'static str] = &["name", "parent_id"];
    fn from_row(row: &Row) -> berserk::database::Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            parent_id: field(row, "parent_id")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
    fn parse_route_key(raw: &str) -> Option<Value> {
        raw.parse::<i64>().ok().map(Into::into)
    }
}
impl ApiResource for User {
    fn to_resource(&self) -> Json {
        Json::Object(
            [
                ("id".into(), self.id.into()),
                ("name".into(), self.name.clone().into()),
            ]
            .into(),
        )
    }
}
struct Input {
    name: String,
}
impl FromJson for Input {
    fn from_json(json: &Json) -> std::result::Result<Self, ValidationErrors> {
        let mut errors = ValidationErrors::default();
        let name = json.get("name").and_then(Json::as_str);
        errors.required("name", name);
        errors.finish()?;
        Ok(Self {
            name: name.unwrap_or_default().into(),
        })
    }
}
impl FormRequest for Input {
    fn sanitize(&mut self) {
        self.name = self.name.trim().into();
    }
    fn validate(&self) -> ValidationResult {
        let mut errors = ValidationErrors::default();
        errors.length("name", &self.name, 1, 20);
        errors.finish()
    }
    fn authorize(&self) -> bool {
        self.name != "forbidden"
    }
}
impl IntoInsert<User> for Input {
    fn into_insert(self) -> berserk::database::Result<Vec<(String, Value)>> {
        Ok(vec![("name".into(), self.name.into())])
    }
}
impl IntoUpdate<User> for Input {
    fn into_update(self) -> berserk::database::Result<Vec<(String, Value)>> {
        self.into_insert()
    }
}
fn database(acquired: Arc<AtomicUsize>) -> Database {
    Database::new(move || {
        acquired.fetch_add(1, Ordering::SeqCst);
        let mut c = SqliteConnection::in_memory()?;
        c.execute(&Statement::new("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, parent_id INTEGER)"))?;
        c.execute(&Statement::new(
            "INSERT INTO users (id,name,parent_id) VALUES (1,'Ada',NULL),(2,'Grace',1)",
        ))?;
        Ok(c)
    })
}
fn request(method: &str, target: &str, body: &str) -> Request {
    let mut headers = Headers::new();
    headers.insert("content-type", "application/json").unwrap();
    Request::new(
        Method::new(method).unwrap(),
        target,
        headers,
        body.as_bytes(),
    )
    .unwrap()
}
fn app() -> (App, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let mut app = App::new();
    app.database(database(count.clone())).unwrap();
    (app, count)
}

#[test]
fn typed_actions_share_one_connection_and_guard_input() -> Result<()> {
    let (mut app, count) = app();
    app.route()
        .post("/users", |input: Input| -> Result<Response> {
            let mut user = User::create(input)?;
            assert_eq!(User::find_or_fail(user.id)?.name, "New");
            user.update(Input {
                name: "Changed".into(),
            })?;
            assert_eq!(user.name, "Changed");
            assert!(User::create([("id", 999_i64.into())]).is_err());
            assert_eq!(User::count()?, 3);
            user.delete()?;
            assert_eq!(User::count()?, 2);
            Ok(Response::no_content())
        })?;
    assert_eq!(
        app.respond(request(
            "POST",
            "/users",
            r#"{"name":" New ","id":999,"admin":true}"#
        ))
        .status_code(),
        204
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(User::all().is_err());
    Ok(())
}
#[test]
fn rejected_forms_never_call_controller_or_acquire_database() -> Result<()> {
    let (mut app, count) = app();
    app.route().post("/users", |_input: Input| -> Response {
        panic!("rejected input reached action")
    })?;
    for (body, status) in [
        (r#"{"name":" "}"#, 422),
        (r#"{"name":"forbidden"}"#, 403),
        ("{", 400),
        (r#"{"name":8}"#, 422),
    ] {
        let response = app.respond(request("POST", "/users", body));
        assert_eq!(response.status_code(), status);
        assert_eq!(
            response.headers().get("content-type"),
            Some("application/json")
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    Ok(())
}
#[test]
fn transactions_commit_rollback_and_reject_nesting() -> Result<()> {
    let scope = DatabaseScope::new(database(Arc::new(AtomicUsize::new(0))));
    scope.run(|| -> Result<()> {
        Transaction::run(|| {
            User::create(Input {
                name: "Committed".into(),
            })
        })?;
        let result: Result<()> = Transaction::run(|| {
            User::create(Input {
                name: "Rolled back".into(),
            })?;
            Err(Error::forbidden())
        });
        assert!(result.is_err());
        assert!(!User::where_("name", "Rolled back").exists()?);
        assert!(User::where_("name", "Committed").exists()?);
        let nested: Result<()> = Transaction::run(|| Transaction::run(|| Ok(())));
        assert!(nested.is_err());
        assert_eq!(User::count()?, 3);
        Ok(())
    })
}
#[test]
fn nested_scopes_restore_on_unwind_and_do_not_cross_threads() -> Result<()> {
    let outer = DatabaseScope::new(database(Arc::new(AtomicUsize::new(0))));
    outer.run(|| -> Result<()> {
        User::create(Input {
            name: "Outer".into(),
        })?;
        let result = std::panic::catch_unwind(|| {
            DatabaseScope::new(database(Arc::new(AtomicUsize::new(0)))).run(|| {
                assert_eq!(User::count().unwrap(), 2);
                panic!("scope unwind");
            });
        });
        assert!(result.is_err());
        assert_eq!(User::count()?, 3);
        assert!(std::thread::spawn(User::all).join().unwrap().is_err());
        Ok(())
    })
}
#[test]
fn typed_eager_loading_pagination_and_binding() -> Result<()> {
    let (mut app, _) = app();
    app.route().get("/users", || -> Result<Response> {
        let loaded = User::query()
            .with(HasMany::new("parent_id", User::key, |u: &User| {
                u.parent_id.map_or(Value::Null, Into::into)
            }))
            .get()?;
        assert_eq!(loaded.models.len(), 2);
        assert_eq!(
            loaded.relations.get(&1_i64.into()).unwrap()[0].name,
            "Grace"
        );
        ResourceCollection::page(User::query().paginate(1)?).into_response()
    })?;
    app.route()
        .get("/users/{id}", |user: User| -> Result<Response> {
            berserk::response().resource(user)
        })?;
    assert_eq!(
        app.respond(request("GET", "/users/1", "")).status_code(),
        200
    );
    assert_eq!(
        app.respond(request("GET", "/users/42", "")).status_code(),
        404
    );
    assert_eq!(
        app.respond(request("GET", "/users/bad", "")).status_code(),
        400
    );
    let response = app.respond(request("GET", "/users?page=2", ""));
    let json = Json::parse(response.body()).unwrap();
    assert_eq!(
        json.get("meta").unwrap().get("current_page"),
        Some(&2_u64.into())
    );
    for query in [
        "page=0",
        "page=-1",
        "page=1&page=2",
        "page=x",
        "page=18446744073709551615",
    ] {
        assert_eq!(
            app.respond(request("GET", &format!("/users?{query}"), ""))
                .status_code(),
            400
        );
    }
    Ok(())
}
#[cfg(feature = "async")]
#[test]
fn async_actions_keep_scope_across_awaits_and_isolate_concurrent_requests() -> Result<()> {
    let (mut app, count) = app();
    app.route()
        .post_async("/users", |input: Input| async move {
            let user = User::create(input)?;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert_eq!(User::count()?, 3);
            let inherited = tokio::spawn(async { User::count() }).await.unwrap();
            assert!(inherited.is_err());
            berserk::response().resource(user)
        })?;
    let app = Arc::new(app);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let mut tasks = vec![];
        for index in 0..12 {
            let app = app.clone();
            tasks.push(tokio::spawn(async move {
                let name = format!("User{index}");
                let response = app
                    .handle_async(request(
                        "POST",
                        "/users",
                        &format!(r#"{{"name":"{name}"}}"#),
                    ))
                    .await
                    .unwrap();
                let json = Json::parse(response.body()).unwrap();
                assert_eq!(
                    json.get("data").unwrap().get("name").and_then(Json::as_str),
                    Some(name.as_str())
                );
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 12);
    assert!(User::all().is_err());
    Ok(())
}
#[cfg(feature = "auth")]
#[test]
fn auth_context_is_request_local_and_duplicate_headers_fail_closed() -> Result<()> {
    use berserk::{
        auth::{Guard, Principal},
        Auth, Authenticated,
    };
    struct Accept;
    impl Guard for Accept {
        fn authenticate(&self, token: &str, _: u64) -> berserk::auth::Result<Option<Principal>> {
            Ok(Principal::new(token))
        }
    }
    let mut app = App::new();
    app.route()
        .middleware(Authenticated::new(Accept))
        .get("/private", || {
            Response::text(Auth::user().unwrap().subject())
        })?;
    let mut headers = Headers::new();
    headers.append("authorization", "Bearer ada")?;
    let response = app.respond(Request::new(
        Method::new("GET")?,
        "/private",
        headers.clone(),
        vec![],
    )?);
    assert_eq!(response.body(), b"ada");
    assert!(!Auth::check());
    headers.append("authorization", "Bearer grace")?;
    assert_eq!(
        app.respond(Request::new(
            Method::new("GET")?,
            "/private",
            headers,
            vec![]
        )?)
        .status_code(),
        401
    );
    Ok(())
}
#[test]
fn middleware_preserves_headers_and_redacts_internal_errors() -> Result<()> {
    let mut app = App::new();
    app.middleware(berserk::RequestId);
    app.middleware(berserk::HandleErrors);
    app.route().get("/error", || -> Result<Response> {
        Err(berserk::ConfigError::new("secret", "password=hidden").into())
    })?;
    let response = app.respond(request("GET", "/error", ""));
    assert_eq!(response.status_code(), 500);
    assert!(response.headers().get("x-request-id").is_some());
    assert!(!std::str::from_utf8(response.body())
        .unwrap()
        .contains("hidden"));
    assert_eq!(app.respond(request("HEAD", "/error", "")).body(), b"");
    Ok(())
}

#[test]
fn exclusive_borrows_fail_promptly_and_explicit_scope_restores() -> Result<()> {
    let scope = DatabaseScope::new(database(Arc::new(AtomicUsize::new(0))));
    scope.run(|| -> Result<()> {
        let guard = scope.connection()?;
        assert!(User::count().is_err());
        drop(guard);
        assert_eq!(User::count()?, 2);
        let mut separate = database(Arc::new(AtomicUsize::new(0))).acquire()?;
        berserk::claw::with_scoped_connection(&mut *separate, || -> Result<()> {
            User::create(Input {
                name: "Separate".into(),
            })?;
            assert_eq!(User::count()?, 3);
            Ok(())
        })?;
        assert_eq!(User::count()?, 2);
        Ok(())
    })
}

#[test]
fn transaction_unwind_rolls_back_and_connection_fails_closed() -> Result<()> {
    let path = std::env::temp_dir().join(format!(
        "berserk-transaction-unwind-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut c = SqliteConnection::open(&path)?;
    c.execute(&Statement::new(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, parent_id INTEGER)",
    ))?;
    drop(c);
    let db_path = path.clone();
    let scope = DatabaseScope::new(Database::new(move || SqliteConnection::open(&db_path)));
    scope.run(|| {
        let panic = std::panic::catch_unwind(|| {
            let _: Result<()> = Transaction::run(|| {
                User::create(Input {
                    name: "Uncommitted".into(),
                })?;
                panic!("transaction interrupted");
            });
        });
        assert!(panic.is_err());
        assert!(User::count().is_err());
    });
    drop(scope);
    let mut c = SqliteConnection::open(&path)?;
    assert_eq!(User::count_on(&mut c)?, 0);
    drop(c);
    std::fs::remove_file(path)?;
    Ok(())
}
