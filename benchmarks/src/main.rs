use berserk::{App, Headers, Json, Method, Request, Response};
use std::{
    error::Error,
    hint::black_box,
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn percentile(sorted: &[u128], percent: usize) -> u128 {
    sorted[(sorted.len() * percent).div_ceil(100).saturating_sub(1)]
}
fn measure(
    mut operation: impl FnMut() -> Result<()>,
    name: &str,
    iterations: usize,
    warmup: usize,
) -> Result<()> {
    for _ in 0..warmup {
        operation()?;
    }
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();
    for _ in 0..iterations {
        let one = Instant::now();
        operation()?;
        samples.push(one.elapsed().as_nanos());
    }
    let elapsed = start.elapsed();
    samples.sort_unstable();
    println!(
        "{name},{iterations},{warmup},{:.9},{:.3},{},{},{},{}",
        elapsed.as_secs_f64(),
        iterations as f64 / elapsed.as_secs_f64(),
        percentile(&samples, 50),
        percentile(&samples, 95),
        percentile(&samples, 99),
        samples.last().unwrap()
    );
    Ok(())
}
fn request(path: &str) -> Request {
    Request::new(
        Method::new("GET").unwrap(),
        path,
        Headers::new(),
        Vec::new(),
    )
    .unwrap()
}
struct Stop(berserk::ShutdownHandle);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let scenario = args.get(1).map(String::as_str).unwrap_or("routing");
    let n: usize = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(10000);
    let warmup: usize = args.get(3).map(|s| s.parse()).transpose()?.unwrap_or(1000);
    if n == 0 || n > 10_000_000 || warmup > 1_000_000 {
        return Err("invalid iteration or warmup count".into());
    }
    if !["routing", "json", "tcp"].contains(&scenario) {
        return Err("scenario must be routing, json or tcp".into());
    }
    eprintln!("scenario={scenario}; os={}; arch={}; profile={}; samples contain timer/allocation overhead",std::env::consts::OS,std::env::consts::ARCH,if cfg!(debug_assertions){"debug (do not use as baseline)"}else{"release"});
    println!("scenario,iterations,warmup,elapsed_seconds,operations_per_second,p50_ns,p95_ns,p99_ns,max_ns");
    match scenario {
        "routing" => {
            let mut app = App::new();
            for i in 0..100 {
                app.route().get(&format!("/items/{i}"), || Response::text("ok"))?;
            }
            app.route().get("/users/{id}", |req: Request| {
                Response::text(req.param("id").unwrap())
            })?;
            measure(
                || {
                    let response = app.handle(request("/users/42"))?;
                    if response.body() != b"42" {
                        return Err("incorrect route result".into());
                    }
                    black_box(response);
                    Ok(())
                },
                "routing_101_routes",
                n,
                warmup,
            )
        }
        "json" => {
            let input=br#"{"id":42,"name":"Example","active":true,"tags":["rust","api"],"nested":{"count":5}}"#;
            let expected = Json::parse(input)?.encode()?;
            measure(
                || {
                    let output = Json::parse(black_box(input))?.encode()?;
                    if output != expected {
                        return Err("incorrect JSON result".into());
                    }
                    black_box(output);
                    Ok(())
                },
                "json_parse_encode",
                n,
                warmup,
            )
        }
        "tcp" => {
            let mut app = App::new();
            app.route().get("/", || Response::text("ok"))?;
            let server = app.bind("127.0.0.1:0")?;
            let addr = server.local_addr()?;
            let stop = Stop(server.shutdown_handle());
            let worker = std::thread::spawn(move || server.run());
            let outcome = measure(
                || {
                    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
                    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
                    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
                    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
                    let mut output = Vec::new();
                    stream.take(65536).read_to_end(&mut output)?;
                    if !output.starts_with(b"HTTP/1.1 200 ") || !output.ends_with(b"\r\n\r\nok") {
                        return Err("incorrect TCP response".into());
                    }
                    black_box(output);
                    Ok(())
                },
                "tcp_sequential_new_connection",
                n,
                warmup,
            );
            drop(stop);
            worker.join().map_err(|_| "server thread panicked")??;
            outcome
        }
        _ => unreachable!(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentiles_use_nearest_rank() {
        assert_eq!(percentile(&[1, 2, 3, 4], 50), 2);
        assert_eq!(percentile(&[1, 2, 3, 4], 95), 4);
        assert_eq!(percentile(&[7], 99), 7);
    }
}
