#![cfg(feature = "view")]

use berserk::{response, view, view_data, Response};

fn write_view(name: &str, source: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from("app")
        .join("views")
        .join(format!("{name}.html"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, source).unwrap();
    path
}

#[test]
fn response_view_resolves_named_template_and_only_receives_explicit_data() {
    let path = write_view(
        "__axe_tests/response",
        "<h1>{{ title }}</h1>@if(show)<p>Visible</p>@endif",
    );

    let response = Response::view(
        "__axe_tests/response",
        view_data!["title" => "<Users>", "show" => true],
    )
    .unwrap();

    assert_eq!(
        response.headers().get("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.body(), b"<h1>&lt;Users&gt;</h1><p>Visible</p>");
    let _ = std::fs::remove_file(path);
}

#[test]
fn view_helper_uses_named_template() {
    let path = write_view("__axe_tests/helper", "<p>{{ message }}</p>");

    let response = view("__axe_tests/helper")
        .with(view_data!["message" => "Hello"])
        .unwrap();

    assert_eq!(response.body(), b"<p>Hello</p>");
    let _ = std::fs::remove_file(path);
}

#[test]
fn view_helper_can_render_without_data() {
    let path = write_view("__axe_tests/empty", "<p>Static</p>");

    let response = view("__axe_tests/empty").render().unwrap();

    assert_eq!(response.body(), b"<p>Static</p>");
    let _ = std::fs::remove_file(path);
}

#[test]
fn response_factory_can_render_named_view() {
    let path = write_view("__axe_tests/factory", "<main>{{ title }}</main>");

    let response = response()
        .status(201)
        .view("__axe_tests/factory", view_data!["title" => "Dashboard"])
        .unwrap();

    assert_eq!(response.status_code(), 201);
    assert_eq!(response.body(), b"<main>Dashboard</main>");
    let _ = std::fs::remove_file(path);
}

#[test]
fn homogeneous_array_data_is_supported_without_macro() {
    let path = write_view("__axe_tests/array", "<strong>{{ title }}</strong>");

    let response = Response::view("__axe_tests/array", [("title", "Users")]).unwrap();

    assert_eq!(response.body(), b"<strong>Users</strong>");
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "claw")]
mod claw_collection {
    use super::write_view;
    use berserk::{
        claw::{Collection, Model, Row, Value},
        view_data, Response,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct User {
        id: i64,
        name: String,
        password_hash: String,
    }

    impl Model for User {
        const TABLE: &'static str = "users";
        const HIDDEN: &'static [&'static str] = &["password_hash"];

        fn from_row(_: &Row) -> berserk::database::Result<Self> {
            unreachable!("database decoding is not used by this view integration test")
        }

        fn key(&self) -> Value {
            self.id.into()
        }

        fn attributes(&self) -> std::collections::BTreeMap<String, Value> {
            [
                ("id".into(), self.id.into()),
                ("name".into(), self.name.clone().into()),
                ("password_hash".into(), self.password_hash.clone().into()),
            ]
            .into()
        }
    }

    #[test]
    fn claw_collection_maps_automatically_into_view_data() {
        let path = write_view(
            "__axe_tests/claw_collection",
            "@foreach(user in users)<p>{{ user.name }}</p>@endforeach",
        );
        let users = Collection::from(vec![
            User {
                id: 1,
                name: "Ada".into(),
                password_hash: "secret-a".into(),
            },
            User {
                id: 2,
                name: "Linus".into(),
                password_hash: "secret-b".into(),
            },
        ]);

        let response =
            Response::view("__axe_tests/claw_collection", view_data!["users" => users]).unwrap();

        assert_eq!(response.body(), b"<p>Ada</p><p>Linus</p>");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn hidden_model_attributes_are_not_exposed_to_axe() {
        let path = write_view(
            "__axe_tests/claw_hidden",
            "@foreach(user in users){{ user.password_hash }}@endforeach",
        );
        let users = Collection::from(vec![User {
            id: 1,
            name: "Ada".into(),
            password_hash: "secret".into(),
        }]);

        let error =
            Response::view("__axe_tests/claw_hidden", view_data!["users" => users]).unwrap_err();

        assert!(error.to_string().contains("user.password_hash"));
        let _ = std::fs::remove_file(path);
    }
}
