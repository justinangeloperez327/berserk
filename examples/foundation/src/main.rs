use berserk::{App, Request, Response, Result};
fn main() -> Result<()> {
    let mut app = App::new();
    app.get("/", || Response::text("Hello, world!"))?;
    app.get("/users/{id}", |req: Request| {
        Response::text(format!("User {}", req.param("id").unwrap_or("")))
    })?;
    app.post("/echo", |req: Request| Response::bytes(req.body().to_vec()))?;
    app.listen("127.0.0.1:3000")
}
