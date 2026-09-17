use super::{hyper_adapter, timeout_io::WriteTimeoutIo, ProtocolError};
use crate::App;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

const HYPER_MIN_BUFFER: usize = 8192;

pub(super) async fn serve(stream: tokio::net::TcpStream, accepted: Instant, app: Arc<App>) -> bool {
    let config = app.config().clone();
    let Some(first_deadline) = accepted.checked_add(config.request_deadline) else {
        return false;
    };
    let first_deadline = tokio::time::Instant::from_std(first_deadline);
    let first_remaining = first_deadline.saturating_duration_since(tokio::time::Instant::now());
    if first_remaining.is_zero() {
        return false;
    }

    let request_count = Arc::new(AtomicUsize::new(0));
    let service = service_fn({
        let app = Arc::clone(&app);
        let request_count = Arc::clone(&request_count);
        let config = config.clone();
        move |request| {
            let app = Arc::clone(&app);
            let request_count = Arc::clone(&request_count);
            let config = config.clone();
            async move {
                let current = request_count.fetch_add(1, Ordering::Relaxed) + 1;
                let deadline = if current == 1 {
                    first_deadline
                } else {
                    tokio::time::Instant::now() + config.request_deadline
                };
                let mut response = hyper_adapter::dispatch(app, request, deadline).await?;
                let connection =
                    if config.keep_alive && current < config.max_requests_per_connection {
                        "keep-alive"
                    } else {
                        "close"
                    };
                response.headers_mut().insert(
                    http::header::CONNECTION,
                    http::HeaderValue::from_static(connection),
                );
                Ok::<_, ProtocolError>(response)
            }
        }
    });

    let mut http = hyper::server::conn::http1::Builder::new();
    http.keep_alive(config.keep_alive)
        .max_headers(config.max_headers)
        .max_buf_size(config.max_header_bytes.max(HYPER_MIN_BUFFER))
        .header_read_timeout(config.read_timeout.min(first_remaining))
        .timer(TokioTimer::new())
        .auto_date_header(false);

    let stream = WriteTimeoutIo::new(stream, config.write_timeout);
    http.serve_connection(TokioIo::new(stream), service)
        .await
        .is_ok()
}
