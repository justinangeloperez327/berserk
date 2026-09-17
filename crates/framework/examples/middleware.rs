use berserk::{App, Next, Request, RequestId, Response, Result};
fn main() -> Result<()> {
    let mut app = App::new();
    app.state(String::from("Example API"))?;
    app.middleware(RequestId);
    app.middleware(|request: Request, next: Next<'_>| -> Result<Response> {
        next.run(request)?.header("x-service", "example")
    });
    app.group("/api", |routes| {
        routes.get("/name", |req: Request| {
            Response::text(req.state::<String>().unwrap().clone())
        })
    })?;
    app.listen("127.0.0.1:3000")
}
