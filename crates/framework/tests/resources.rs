use framework::{ApiResource, IntoResponse, Json, Resource, ResourceCollection};
use std::collections::BTreeMap;

struct User {
    id: u64,
    name: String,
    password_hash: String,
}

impl ApiResource for User {
    fn to_resource(&self) -> Json {
        Json::Object(BTreeMap::from([
            ("id".into(), self.id.into()),
            ("name".into(), self.name.clone().into()),
        ]))
    }
}

#[test]
fn resources_expose_only_explicit_public_fields() {
    let user = User {
        id: 7,
        name: "Ada".into(),
        password_hash: "never-public".into(),
    };
    assert_eq!(user.password_hash, "never-public");
    let response = Resource::new(user).into_response().unwrap();
    let body = std::str::from_utf8(response.body()).unwrap();
    assert_eq!(
        response.headers().get("content-type"),
        Some("application/json")
    );
    assert_eq!(body, r#"{"data":{"id":7,"name":"Ada"}}"#);
    assert!(!body.contains("password"));
}

#[test]
fn collections_include_optional_links_and_metadata() {
    let users = vec![User {
        id: 1,
        name: "A".into(),
        password_hash: "x".into(),
    }];
    let response = ResourceCollection::new(users)
        .link("next", "/users?page=2")
        .meta("total", 10_u64)
        .into_response()
        .unwrap();
    let json = Json::parse(response.body()).unwrap();
    assert_eq!(
        json.get("meta").unwrap().get("total"),
        Some(&Json::from(10_u64))
    );
    assert_eq!(
        json.get("links")
            .unwrap()
            .get("next")
            .and_then(Json::as_str),
        Some("/users?page=2")
    );
}
