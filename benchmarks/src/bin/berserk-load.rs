use berserk::server::{ServerSnapshot, ServerStats};
use berserk::{App, Response, ServerConfig, ShutdownHandle};
use std::{
    error::Error,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Barrier,
    },
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

struct RunningServer {
    address: SocketAddr,
    shutdown: ShutdownHandle,
    stats: ServerStats,
    thread: Option<thread::JoinHandle<std::io::Result<()>>>,
}

impl RunningServer {
    fn start(app: App) -> Result<Self> {
        let server = app.bind("127.0.0.1:0")?;
        let address = server.local_addr()?;
        let shutdown = server.shutdown_handle();
        let stats = server.stats();
        let thread = thread::spawn(move || server.run());
        Ok(Self {
            address,
            shutdown,
            stats,
            thread: Some(thread),
        })
    }

    fn stop(mut self) -> Result<ServerSnapshot> {
        self.shutdown.shutdown();
        let result = self
            .thread
            .take()
            .ok_or("server thread already joined")?
            .join()
            .map_err(|_| "server thread panicked")?;
        result?;
        Ok(self.stats.snapshot())
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.shutdown.shutdown();
    }
}

#[derive(Debug)]
struct Metrics {
    scenario: &'static str,
    attempted: usize,
    successes: usize,
    errors: usize,
    snapshot: ServerSnapshot,
    elapsed: Duration,
    latencies_us: Vec<u128>,
}

impl Metrics {
    fn print_csv(&mut self) {
        self.latencies_us.sort_unstable();
        let p50 = percentile(&self.latencies_us, 50);
        let p95 = percentile(&self.latencies_us, 95);
        let p99 = percentile(&self.latencies_us, 99);
        let max = self.latencies_us.last().copied().unwrap_or(0);
        let seconds = self.elapsed.as_secs_f64();
        let rps = if seconds > 0.0 {
            self.successes as f64 / seconds
        } else {
            0.0
        };
        println!(
            "{},{},{},{},{},{},{},{},{:.6},{:.3},{},{},{},{}",
            self.scenario,
            self.attempted,
            self.successes,
            self.errors,
            self.snapshot.accepted,
            self.snapshot.rejected,
            self.snapshot.completed,
            self.snapshot.failed,
            seconds,
            rps,
            p50,
            p95,
            p99,
            max
        );
    }
}

fn percentile(sorted: &[u128], percent: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[(sorted.len() * percent).div_ceil(100).saturating_sub(1)]
}

fn request(address: SocketAddr) -> Result<Duration> {
    let started = Instant::now();
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
    let mut output = Vec::new();
    stream.take(65_536).read_to_end(&mut output)?;
    if !output.starts_with(b"HTTP/1.1 200 ") || !output.ends_with(b"\r\n\r\nok") {
        return Err("unexpected HTTP response".into());
    }
    Ok(started.elapsed())
}

fn wait_until(mut ready: impl FnMut() -> bool, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while !ready() {
        if Instant::now() >= deadline {
            return Err("condition timed out".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn run_fixed_load(total: usize, concurrency: usize) -> Result<Metrics> {
    let mut app = App::new();
    app.get("/", || Response::text("ok"))?;
    let server = RunningServer::start(app)?;
    let next = Arc::new(AtomicUsize::new(0));
    let started = Instant::now();
    let mut workers = Vec::with_capacity(concurrency);

    for _ in 0..concurrency {
        let next = Arc::clone(&next);
        let address = server.address;
        workers.push(thread::spawn(move || {
            let mut latencies = Vec::new();
            let mut errors = 0;
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= total {
                    break;
                }
                match request(address) {
                    Ok(latency) => latencies.push(latency.as_micros()),
                    Err(_) => errors += 1,
                }
            }
            (latencies, errors)
        }));
    }

    let mut latencies = Vec::with_capacity(total);
    let mut errors = 0;
    for worker in workers {
        let (mut local, local_errors) = worker.join().map_err(|_| "load worker panicked")?;
        latencies.append(&mut local);
        errors += local_errors;
    }
    let elapsed = started.elapsed();
    wait_until(
        || {
            let s = server.stats.snapshot();
            s.completed + s.failed + s.rejected >= s.accepted
        },
        Duration::from_secs(5),
    )?;
    let snapshot = server.stop()?;
    let successes = latencies.len();
    if successes + errors != total {
        return Err("fixed-load accounting mismatch".into());
    }
    if errors != 0 || snapshot.rejected != 0 || snapshot.failed != 0 || snapshot.completed != total {
        return Err(format!("steady load was not lossless: {snapshot:?}, client_errors={errors}").into());
    }
    Ok(Metrics {
        scenario: "concurrent_steady",
        attempted: total,
        successes,
        errors,
        snapshot,
        elapsed,
        latencies_us: latencies,
    })
}

fn run_overload(concurrency: usize) -> Result<Metrics> {
    let mut app = App::with_config(ServerConfig {
        workers: 1,
        queue_capacity: 2,
        request_deadline: Duration::from_secs(5),
        ..ServerConfig::default()
    })?;
    app.get("/", || {
        thread::sleep(Duration::from_millis(75));
        Response::text("ok")
    })?;
    let server = RunningServer::start(app)?;
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let started = Instant::now();
    let mut workers = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let barrier = Arc::clone(&barrier);
        let address = server.address;
        workers.push(thread::spawn(move || {
            barrier.wait();
            request(address)
        }));
    }
    barrier.wait();

    let mut latencies = Vec::new();
    let mut errors = 0;
    for worker in workers {
        match worker.join().map_err(|_| "overload worker panicked")? {
            Ok(latency) => latencies.push(latency.as_micros()),
            Err(_) => errors += 1,
        }
    }
    let elapsed = started.elapsed();
    wait_until(
        || {
            let s = server.stats.snapshot();
            s.completed + s.failed + s.rejected >= s.accepted
        },
        Duration::from_secs(8),
    )?;
    let snapshot = server.stop()?;
    if snapshot.rejected == 0 {
        return Err("overload scenario did not exercise queue rejection".into());
    }
    if snapshot.completed == 0 {
        return Err("overload scenario completed no work".into());
    }
    Ok(Metrics {
        scenario: "overload_bounded_queue",
        attempted: concurrency,
        successes: latencies.len(),
        errors,
        snapshot,
        elapsed,
        latencies_us: latencies,
    })
}

fn run_shutdown_under_load(concurrency: usize) -> Result<Metrics> {
    let mut app = App::with_config(ServerConfig {
        workers: 4,
        queue_capacity: 32,
        request_deadline: Duration::from_secs(5),
        ..ServerConfig::default()
    })?;
    app.get("/", || {
        thread::sleep(Duration::from_millis(20));
        Response::text("ok")
    })?;
    let server = RunningServer::start(app)?;
    let stop = Arc::new(AtomicBool::new(false));
    let attempted = Arc::new(AtomicUsize::new(0));
    let started = Instant::now();
    let mut workers = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let stop = Arc::clone(&stop);
        let attempted = Arc::clone(&attempted);
        let address = server.address;
        workers.push(thread::spawn(move || {
            let mut latencies = Vec::new();
            let mut errors = 0;
            while !stop.load(Ordering::Relaxed) {
                attempted.fetch_add(1, Ordering::Relaxed);
                match request(address) {
                    Ok(latency) => latencies.push(latency.as_micros()),
                    Err(_) => errors += 1,
                }
            }
            (latencies, errors)
        }));
    }

    wait_until(
        || server.stats.snapshot().accepted >= concurrency * 2,
        Duration::from_secs(5),
    )?;
    server.shutdown.shutdown();
    stop.store(true, Ordering::Relaxed);

    let mut latencies = Vec::new();
    let mut errors = 0;
    for worker in workers {
        let (mut local, local_errors) = worker.join().map_err(|_| "shutdown worker panicked")?;
        latencies.append(&mut local);
        errors += local_errors;
    }
    let elapsed = started.elapsed();
    let attempted = attempted.load(Ordering::Relaxed);
    let snapshot = server.stop()?;
    if snapshot.completed + snapshot.failed + snapshot.rejected != snapshot.accepted {
        return Err(format!("server did not drain accepted work: {snapshot:?}").into());
    }
    if snapshot.completed == 0 {
        return Err("shutdown scenario completed no work".into());
    }
    Ok(Metrics {
        scenario: "shutdown_under_load",
        attempted,
        successes: latencies.len(),
        errors,
        snapshot,
        elapsed,
        latencies_us: latencies,
    })
}

fn run_soak(seconds: u64, concurrency: usize) -> Result<Metrics> {
    let mut app = App::new();
    app.get("/", || Response::text("ok"))?;
    let server = RunningServer::start(app)?;
    let stop = Arc::new(AtomicBool::new(false));
    let attempted = Arc::new(AtomicUsize::new(0));
    let started = Instant::now();
    let mut workers = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let stop = Arc::clone(&stop);
        let attempted = Arc::clone(&attempted);
        let address = server.address;
        workers.push(thread::spawn(move || {
            let mut latencies = Vec::new();
            let mut errors = 0;
            while !stop.load(Ordering::Relaxed) {
                attempted.fetch_add(1, Ordering::Relaxed);
                match request(address) {
                    Ok(latency) => latencies.push(latency.as_micros()),
                    Err(_) => errors += 1,
                }
            }
            (latencies, errors)
        }));
    }
    thread::sleep(Duration::from_secs(seconds));
    stop.store(true, Ordering::Relaxed);

    let mut latencies = Vec::new();
    let mut errors = 0;
    for worker in workers {
        let (mut local, local_errors) = worker.join().map_err(|_| "soak worker panicked")?;
        latencies.append(&mut local);
        errors += local_errors;
    }
    let elapsed = started.elapsed();
    wait_until(
        || {
            let s = server.stats.snapshot();
            s.completed + s.failed + s.rejected >= s.accepted
        },
        Duration::from_secs(5),
    )?;
    let snapshot = server.stop()?;
    let attempted = attempted.load(Ordering::Relaxed);
    if errors != 0 || snapshot.rejected != 0 || snapshot.failed != 0 {
        return Err(format!("soak encountered errors: {snapshot:?}, client_errors={errors}").into());
    }
    if snapshot.completed != attempted {
        return Err(format!("soak accounting mismatch: attempted={attempted}, {snapshot:?}").into());
    }
    Ok(Metrics {
        scenario: "concurrent_soak",
        attempted,
        successes: latencies.len(),
        errors,
        snapshot,
        elapsed,
        latencies_us: latencies,
    })
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("smoke");
    println!("scenario,attempted,successes,client_errors,accepted,rejected,completed,failed,elapsed_seconds,requests_per_second,p50_us,p95_us,p99_us,max_us");
    match mode {
        "smoke" => {
            let total = args.get(2).map(|v| v.parse()).transpose()?.unwrap_or(1_000);
            let concurrency = args.get(3).map(|v| v.parse()).transpose()?.unwrap_or(16);
            if total == 0 || concurrency == 0 || concurrency > 256 || total < concurrency {
                return Err("invalid smoke total/concurrency".into());
            }
            let mut steady = run_fixed_load(total, concurrency)?;
            steady.print_csv();
            let mut overload = run_overload(concurrency.max(16))?;
            overload.print_csv();
            let mut shutdown = run_shutdown_under_load(concurrency.min(64))?;
            shutdown.print_csv();
        }
        "soak" => {
            let seconds = args.get(2).map(|v| v.parse()).transpose()?.unwrap_or(60);
            let concurrency = args.get(3).map(|v| v.parse()).transpose()?.unwrap_or(16);
            if seconds == 0 || seconds > 3_600 || concurrency == 0 || concurrency > 256 {
                return Err("invalid soak duration/concurrency".into());
            }
            let mut soak = run_soak(seconds, concurrency)?;
            soak.print_csv();
        }
        _ => return Err("mode must be smoke or soak".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_nearest_rank_and_handles_empty_input() {
        assert_eq!(percentile(&[], 95), 0);
        assert_eq!(percentile(&[1, 2, 3, 4], 50), 2);
        assert_eq!(percentile(&[1, 2, 3, 4], 95), 4);
    }
}
