use berserk::prelude::*;
use berserk::{Arr, Headers, Json, Method, Str};

#[test]
fn preferred_application_surface_stays_small_and_explicit() {
    let mut app = App::new();

    app.route()
        .get("/users/{id}", |id: u64| response().text(format!("user:{id}")))
        .unwrap();

    let request = Request::new(
        Method::new("GET").unwrap(),
        "/users/7",
        Headers::new(),
        Vec::new(),
    )
    .unwrap();

    let response = app.respond(request);
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body(), b"user:7");
}

#[test]
fn support_helpers_remain_explicit_root_utilities() {
    assert_eq!(Str::kebab("Berserk Framework"), "berserk-framework");

    let value = Json::Object(
        [(
            "framework".into(),
            Json::Object([("name".into(), Json::from("Berserk"))].into()),
        )]
        .into(),
    );

    assert_eq!(Arr::string(&value, "framework.name"), Some("Berserk"));
}
