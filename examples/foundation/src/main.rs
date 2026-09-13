use framework::{App, Response, Result};
fn main() -> Result<()> {
    let mut app = App::new();
    app.get("/", |_| Response::text("Hello, world!"))?;
    app.get("/users/{id}", |req| {
        Response::text(format!("User {}", req.param("id").unwrap_or("")))
    })?;
    app.post("/echo", |req| Response::bytes(req.body().to_vec()))?;
    app.listen("127.0.0.1:3000")
}
