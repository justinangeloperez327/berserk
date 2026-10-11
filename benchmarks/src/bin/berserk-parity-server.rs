//! Stable end-to-end HTTP comparison fixture; run with --release.
//! Match settings and response bytes with a separately built Gungnir fixture.
use berserk::{App, Response, ServerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("BERSERK_BENCH_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let config = ServerConfig {
        keep_alive: true,
        ..ServerConfig::default()
    };
    let mut app = App::with_config(config)?;
    app.route().get("/", || Response::text("ok"))?;
    println!("Berserk parity fixture listening on {address}");
    app.listen(address)?;
    Ok(())
}
