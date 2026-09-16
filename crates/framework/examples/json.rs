use framework::{App, FromJson, Json, Request, Response, Result, ValidateInput, ValidationErrors};
struct CreateUser {
    name: String,
}
impl FromJson for CreateUser {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        match value.get("name").and_then(Json::as_str) {
            Some(name) => Ok(Self { name: name.into() }),
            None => {
                let mut e = ValidationErrors::default();
                e.add("name", "string", "Expected a string.");
                Err(e)
            }
        }
    }
}
impl ValidateInput for CreateUser {
    fn validate(&self) -> std::result::Result<(), ValidationErrors> {
        let mut e = ValidationErrors::default();
        e.required("name", Some(&self.name));
        e.length("name", &self.name, 1, 100);
        e.finish()
    }
}
fn main() -> Result<()> {
    let mut app = App::new();
    app.post("/users", |req: Request| -> Result<Response> {
        let input: CreateUser = req.validated()?;
        Response::json(&Json::Object(
            [("name".into(), Json::String(input.name))]
                .into_iter()
                .collect(),
        ))
        .map(|r| r.status(201))
    })?;
    app.listen("127.0.0.1:3000")
}
