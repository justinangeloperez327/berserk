use framework_openapi::{
    ApiResponse, HttpMethod, Info, OpenApi, Operation, Parameter, ParameterLocation, RequestBody,
    Schema, SchemaRef, SecurityScheme,
};

fn user_schema() -> Schema {
    Schema::object()
        .property("id", Schema::integer().format("int64"))
        .unwrap()
        .property("name", Schema::string())
        .unwrap()
        .required("id")
        .unwrap()
        .required("name")
        .unwrap()
}

#[test]
fn document_contains_explicit_schemas_operations_and_security() {
    let mut api = OpenApi::new(
        Info::new("Users API", "1.0.0")
            .unwrap()
            .description("Example API"),
    );
    api.schema("User", user_schema()).unwrap();
    api.security_scheme("bearerAuth", SecurityScheme::bearer(Some("opaque")))
        .unwrap();

    let operation = Operation::new("showUser")
        .unwrap()
        .summary("Show one user")
        .tag("Users")
        .parameter(Parameter::new("id", ParameterLocation::Path, Schema::integer()).unwrap())
        .unwrap()
        .response(
            "200",
            ApiResponse::new("User found")
                .unwrap()
                .json(SchemaRef::named("User").unwrap()),
        )
        .unwrap()
        .response("404", ApiResponse::new("User not found").unwrap())
        .unwrap()
        .secured_by("bearerAuth", std::iter::empty::<String>())
        .unwrap();
    api.operation(HttpMethod::Get, "/users/{id}", operation)
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&api.to_json().unwrap()).unwrap();
    assert_eq!(value["openapi"], "3.1.0");
    assert_eq!(
        value["paths"]["/users/{id}"]["get"]["operationId"],
        "showUser"
    );
    assert_eq!(value["components"]["schemas"]["User"]["required"][0], "id");
    assert_eq!(
        value["components"]["securitySchemes"]["bearerAuth"]["type"],
        "http"
    );
}

#[test]
fn request_bodies_and_response_contracts_are_serialized() {
    let create = Schema::object()
        .property("name", Schema::string())
        .unwrap()
        .required("name")
        .unwrap();
    let operation = Operation::new("createUser")
        .unwrap()
        .request_body(RequestBody::json(create))
        .unwrap()
        .response(
            "201",
            ApiResponse::new("Created")
                .unwrap()
                .json(SchemaRef::named("User").unwrap()),
        )
        .unwrap();
    let mut api = OpenApi::new(Info::new("Users", "1").unwrap());
    api.schema("User", user_schema()).unwrap();
    api.operation(HttpMethod::Post, "/users", operation)
        .unwrap();
    let json = api.to_json().unwrap();
    assert!(json.contains("requestBody"));
    assert!(json.contains("application/json"));
}

#[test]
fn path_parameters_must_exactly_match_the_template() {
    let operation = Operation::new("showUser")
        .unwrap()
        .response("200", ApiResponse::new("OK").unwrap())
        .unwrap();
    let mut api = OpenApi::new(Info::new("Users", "1").unwrap());
    assert!(api
        .operation(HttpMethod::Get, "/users/{id}", operation)
        .is_err());
}

#[test]
fn duplicate_contracts_and_invalid_statuses_fail_during_setup() {
    let mut api = OpenApi::new(Info::new("Users", "1").unwrap());
    api.schema("User", user_schema()).unwrap();
    assert!(api.schema("User", user_schema()).is_err());
    assert!(Operation::new("bad")
        .unwrap()
        .response("999", ApiResponse::new("Bad").unwrap())
        .is_err());

    let first = Operation::new("sameId")
        .unwrap()
        .response("200", ApiResponse::new("OK").unwrap())
        .unwrap();
    let second = Operation::new("sameId")
        .unwrap()
        .response("200", ApiResponse::new("OK").unwrap())
        .unwrap();
    api.operation(HttpMethod::Get, "/first", first).unwrap();
    assert!(api.operation(HttpMethod::Get, "/second", second).is_err());
}

#[test]
fn unresolved_schema_and_security_references_fail_before_export() {
    let operation = Operation::new("listUsers")
        .unwrap()
        .response(
            "200",
            ApiResponse::new("OK")
                .unwrap()
                .json(SchemaRef::named("Missing").unwrap()),
        )
        .unwrap()
        .secured_by("missingAuth", std::iter::empty::<String>())
        .unwrap();
    let mut api = OpenApi::new(Info::new("Users", "1").unwrap());
    api.operation(HttpMethod::Get, "/users", operation).unwrap();
    assert!(api.to_json().is_err());
}
