mod migrations;
mod users;

use berserk::{
    auth::{Guard, MemoryTokenStore, Principal, TokenManager},
    database::{drivers::sqlite::SqliteConnection, Database},
    response, App, HandleErrors, Next, Request, RequestId, Result,
};
use std::sync::Arc;

fn application<G: Guard>(database: Database, guard: G) -> Result<App> {
    let mut app = App::new();
    app.database(database)?;
    app.auth(guard)?;
    app.middleware(RequestId);
    app.middleware(HandleErrors);

    {
        let mut routes = app.route();
        let mut api = routes.middleware(|request: Request, next: Next<'_>| {
            next.run(request)?.header("x-api-version", "1.0")
        });
        api.can("users.manage")?.crud("/users", users::Users)?;
        api.can("users.read")?
            .get("/users/browse", users::Users::browse)?;
        api.auth()
            .get("/users/{id}/policy", users::Users::policy_show)?;
    }

    app.route().get("/health", || response().text("OK"))?;
    Ok(app)
}

fn main() -> Result<()> {
    let path = std::env::var("BERSERK_DATABASE").unwrap_or_else(|_| "foundation.sqlite".into());
    let mut connection = SqliteConnection::open(&path)?;
    migrations::migrate(&mut connection)?;
    drop(connection);

    let tokens = Arc::new(TokenManager::new(MemoryTokenStore::default()));
    let operator = tokens.issue(
        Principal::new("admin:1")
            .expect("static principal")
            .with_role("admin"),
        ["users.manage", "users.read", "users.view"],
        0,
    )?;
    if std::env::var_os("BERSERK_SHOW_DEMO_TOKEN").is_some() {
        eprintln!(
            "Foundation development bearer token (restart invalidates it): {}",
            operator.expose()
        );
    } else {
        eprintln!("Set BERSERK_SHOW_DEMO_TOKEN=1 to print the local development token.");
    }

    application(
        Database::new({
            let path = path.clone();
            move || SqliteConnection::open(&path)
        }),
        tokens,
    )?
    .listen("127.0.0.1:3000")
}

#[cfg(test)]
mod tests {
    use super::*;
    use berserk::{
        auth::ApiToken,
        database::{Query, Value},
        Json,
    };
    use berserk_testing::TestClient;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

    struct TestApplication {
        app: App,
        database_path: PathBuf,
        admin: ApiToken,
        reader: ApiToken,
    }

    impl Drop for TestApplication {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.database_path);
        }
    }

    fn test_application() -> Result<TestApplication> {
        let id = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "berserk-foundation-{}-{id}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let mut connection = SqliteConnection::open(&path)?;
        migrations::migrate(&mut connection)?;
        drop(connection);

        let tokens = Arc::new(TokenManager::new(MemoryTokenStore::default()));
        let admin = tokens.issue(
            Principal::new("admin:1")
                .expect("static principal")
                .with_role("admin"),
            ["users.manage", "users.read", "users.view"],
            0,
        )?;
        let reader = tokens.issue(
            Principal::new("user:1").expect("static principal"),
            ["users.read", "users.view"],
            0,
        )?;

        let database_path = path.clone();
        let app = application(
            Database::new(move || SqliteConnection::open(&database_path)),
            tokens,
        )?;

        Ok(TestApplication {
            app,
            database_path: path,
            admin,
            reader,
        })
    }

    fn user_payload(name: &str, email: &str) -> Json {
        Json::Object(
            [
                ("name".into(), Json::from(name)),
                ("email".into(), Json::from(email)),
            ]
            .into(),
        )
    }

    fn create_user(
        client: &TestClient<'_>,
        token: &ApiToken,
        name: &str,
        email: &str,
    ) -> Result<i64> {
        let response = client
            .post("/users")?
            .bearer(token.expose())?
            .json(&user_payload(name, email))?
            .send()?
            .assert_created()
            .assert_header("x-api-version", "1.0");
        let json = response.json();
        let id = match json.get("id") {
            Some(Json::Number(number)) => number.as_i64().expect("created user id"),
            _ => panic!("created user id"),
        };
        Ok(id)
    }

    fn seed_relationships(path: &std::path::Path, user_id: i64) -> Result<()> {
        let mut connection = SqliteConnection::open(path)?;
        Query::table("posts")
            .insert([
                ("id", Value::from(10_i64)),
                ("user_id", Value::from(user_id)),
                ("title", Value::from("First post")),
            ])
            .execute(&mut connection)?;
        Query::table("profiles")
            .insert([
                ("id", Value::from(20_i64)),
                ("user_id", Value::from(user_id)),
            ])
            .execute(&mut connection)?;
        Query::table("roles")
            .insert([("id", Value::from(30_i64)), ("name", Value::from("editor"))])
            .execute(&mut connection)?;
        Query::table("role_user")
            .insert([
                ("user_id", Value::from(user_id)),
                ("role_id", Value::from(30_i64)),
            ])
            .execute(&mut connection)?;
        Ok(())
    }

    #[test]
    fn authenticated_crud_uses_generated_contract_and_validation() -> Result<()> {
        let fixture = test_application()?;
        let client = TestClient::new(&fixture.app);

        assert_eq!(
            fixture.app.path_for("users.show", &[("id", "7")])?,
            "/users/7"
        );
        client.get("/users")?.assert_unauthorized();
        client
            .post("/users")?
            .bearer(fixture.reader.expose())?
            .json(&user_payload("Reader", "reader@example.com"))?
            .send()?
            .assert_forbidden();

        let user_id = create_user(&client, &fixture.admin, " Ada ", " ADA@EXAMPLE.COM ")?;
        assert_eq!(user_id, 1);

        client
            .post("/users")?
            .bearer(fixture.admin.expose())?
            .json(&user_payload("Duplicate", "ada@example.com"))?
            .send()?
            .assert_validation_error("email");

        client
            .request("GET", format!("/users/{user_id}"))?
            .bearer(fixture.admin.expose())?
            .send()?
            .assert_ok()
            .assert_json_path("name", "Ada");

        client
            .put(format!("/users/{user_id}"))?
            .bearer(fixture.admin.expose())?
            .json(&user_payload("Ada Lovelace", "ada@example.com"))?
            .send()?
            .assert_ok()
            .assert_json_path("name", "Ada Lovelace");

        client
            .delete(format!("/users/{user_id}"))?
            .bearer(fixture.admin.expose())?
            .send()?
            .assert_no_content();

        client
            .request("GET", format!("/users/{user_id}"))?
            .bearer(fixture.admin.expose())?
            .send()?
            .assert_not_found();

        Ok(())
    }

    #[test]
    fn relationships_policy_and_axe_view_run_together() -> Result<()> {
        let fixture = test_application()?;
        let client = TestClient::new(&fixture.app);
        let first = create_user(&client, &fixture.admin, "Ada", "ada@example.com")?;
        let second = create_user(&client, &fixture.admin, "Grace", "grace@example.com")?;
        seed_relationships(&fixture.database_path, first)?;

        let listing = client
            .request("GET", "/users")?
            .bearer(fixture.admin.expose())?
            .send()?
            .assert_ok();
        let json = listing.json();
        let data = match json.get("data") {
            Some(Json::Array(data)) => data,
            _ => panic!("paginated user data"),
        };
        let ada = match &data[0] {
            Json::Object(user) => user,
            _ => panic!("user object"),
        };
        let posts = match ada.get("posts") {
            Some(Json::Array(posts)) => posts,
            _ => panic!("posts relation"),
        };
        assert_eq!(
            posts[0].get("title").and_then(Json::as_str),
            Some("First post")
        );
        assert!(matches!(ada.get("profile"), Some(Json::Object(_))));
        let roles = match ada.get("roles") {
            Some(Json::Array(roles)) => roles,
            _ => panic!("roles relation"),
        };
        assert_eq!(roles[0].get("name").and_then(Json::as_str), Some("editor"));

        client
            .request("GET", "/users/browse")?
            .bearer(fixture.reader.expose())?
            .send()?
            .assert_ok()
            .assert_header("content-type", "text/html; charset=utf-8")
            .assert_text_contains("Ada")
            .assert_text_contains("First post")
            .assert_text_contains("editor");

        client
            .request("GET", format!("/users/{first}/policy"))?
            .bearer(fixture.reader.expose())?
            .send()?
            .assert_ok();
        client
            .request("GET", format!("/users/{second}/policy"))?
            .bearer(fixture.reader.expose())?
            .send()?
            .assert_forbidden();
        client
            .request("GET", format!("/users/{second}/policy"))?
            .bearer(fixture.admin.expose())?
            .send()?
            .assert_ok();

        Ok(())
    }
}
