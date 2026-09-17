// Phase 1 design example. Not runnable until the framework is implemented.
use berserk::{App, Request, Response, Result};

fn main() -> Result<()> {
    let mut app = App::new();

    app.get("/", |_req| Response::text("Hello, world!"))?;
    app.get("/users/{id}", show_user)?;
    app.post("/echo", |req| Response::bytes(req.body().to_vec()))?;
    app.post("/echo-text", echo_text)?;

    app.listen("127.0.0.1:3000")
}

fn show_user(req: Request) -> Response {
    match req.param("id") {
        Some(id) => Response::text(format!("User {id}")),
        None => Response::text("Missing user ID").status(400),
    }
}

fn echo_text(req: Request) -> Result<Response> {
    Ok(Response::text(req.text()?))
}
