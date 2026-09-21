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

    let response = view("__axe_tests/helper", view_data!["message" => "Hello"]).unwrap();

    assert_eq!(response.body(), b"<p>Hello</p>");
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
