mod users;
use berserk::{
    database::{drivers::sqlite::SqliteConnection, Connection, Database, Statement},
    response, App, HandleErrors, Next, Request, RequestId, Result,
};

fn application(database: Database) -> Result<App> {
    let mut app = App::new();
    app.database(database)?;
    app.middleware(RequestId);
    app.middleware(HandleErrors);
    app.route()
        .middleware(|request: Request, next: Next<'_>| {
            next.run(request)?.header("x-api-version", "0.3")
        })
        .group(|routes| {
            routes.get("/users", users::Users::index)?;
            routes.get("/users/browse", users::Users::browse)?;
            routes.post("/users", users::Users::store)?;
            routes.get("/users/{id}", users::Users::show)?;
            routes.put("/users/{id}", users::Users::update)?;
            routes.patch("/users/{id}", users::Users::update)?;
            routes.delete("/users/{id}", users::Users::destroy)
        })?;
    app.route().get("/health", || response().text("OK"))?;
    Ok(app)
}
fn main() -> Result<()> {
    let path = std::env::var("BERSERK_DATABASE").unwrap_or_else(|_| "foundation.sqlite".into());
    let mut connection = SqliteConnection::open(&path)?;
    connection.execute(&Statement::new("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE)"))?;
    application(Database::new(move || SqliteConnection::open(&path)))?.listen("127.0.0.1:3000")
}

#[cfg(test)]
mod tests {
    use super::*;
    use berserk::{Headers, Json, Method};
    #[test]
    fn typed_crud_sanitizes_and_rejects_duplicate_email() -> Result<()> {
        let path =
            std::env::temp_dir().join(format!("berserk-foundation-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut c = SqliteConnection::open(&path)?;
        c.execute(&Statement::new("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE)"))?;
        drop(c);
        let db_path = path.clone();
        let app = application(Database::new(move || SqliteConnection::open(&db_path)))?;
        let send = |method: &str, target: &str, body: &str| {
            let mut headers = Headers::new();
            headers.insert("content-type", "application/json").unwrap();
            app.respond(
                Request::new(
                    Method::new(method).unwrap(),
                    target,
                    headers,
                    body.as_bytes(),
                )
                .unwrap(),
            )
        };
        let created = send(
            "POST",
            "/users",
            r#"{"name":" Ada ","email":" ADA@EXAMPLE.COM ","id":999}"#,
        );
        assert_eq!(created.status_code(), 201);
        let json = Json::parse(created.body()).unwrap();
        assert_eq!(json.get("name").and_then(Json::as_str), Some("Ada"));
        assert_eq!(
            send(
                "POST",
                "/users",
                r#"{"name":"Ada","email":"ada@example.com"}"#
            )
            .status_code(),
            422
        );
        assert_eq!(send("GET", "/users", "").status_code(), 200);
        assert_eq!(send("GET", "/users/999", "").status_code(), 404);
        assert_eq!(send("DELETE", "/users/1", "").status_code(), 204);
        drop(app);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
