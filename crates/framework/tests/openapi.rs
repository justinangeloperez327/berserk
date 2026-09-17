#![cfg(feature = "openapi")]

use berserk::{
    openapi::{ApiResponse, HttpMethod, Info, OpenApi, Operation},
    App, Headers, Method, Request, Response,
};

#[test]
fn documented_routes_register_runtime_and_contract_together() {
    let mut app = App::new();
    let mut document = OpenApi::new(Info::new("Health API", "1").unwrap());
    let operation = Operation::new("healthCheck")
        .unwrap()
        .response("200", ApiResponse::new("Healthy").unwrap())
        .unwrap();
    app.documented_route(&mut document, HttpMethod::Get, "/health", operation, || {
        Response::text("OK")
    })
    .unwrap();

    let request = Request::new(
        Method::new("GET").unwrap(),
        "/health",
        Headers::new(),
        vec![],
    )
    .unwrap();
    assert_eq!(app.handle(request).unwrap().body(), b"OK");
    let json = document.to_json().unwrap();
    assert!(json.contains("healthCheck"));
    assert!(json.contains("/health"));
}
