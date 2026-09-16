use super::hyper_adapter;
use crate::App;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const HYPER_MIN_BUFFER: usize = 8192;

pub(super) async fn serve(stream: tokio::net::TcpStream, app: Arc<App>) -> bool {
    let config = app.config().clone();
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
                let mut response = hyper_adapter::dispatch(app, request).await;
                let connection = if config.keep_alive
                    && current < config.max_requests_per_connection
                {
                    "keep-alive"
                } else {
                    "close"
                };
                response.headers_mut().insert(
                    http::header::CONNECTION,
                    http::HeaderValue::from_static(connection),
                );
                Ok::<_, Infallible>(response)
            }
        }
    });

    let mut http = hyper::server::conn::http1::Builder::new();
    http.keep_alive(config.keep_alive)
        .max_headers(config.max_headers)
        .max_buf_size(config.max_header_bytes.max(HYPER_MIN_BUFFER))
        .header_read_timeout(config.read_timeout.min(config.request_deadline))
        .timer(TokioTimer::new())
        .auto_date_header(false);

    http.serve_connection(TokioIo::new(stream), service)
        .await
        .is_ok()
}
