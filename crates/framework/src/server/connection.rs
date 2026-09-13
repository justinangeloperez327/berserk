use super::{read_request, write_response_connection, ProtocolError};
use crate::{App, Error, HttpError, Method, Response};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    panic::{catch_unwind, AssertUnwindSafe},
    time::{Duration, Instant},
};

struct Transport<'a> {
    stream: &'a mut TcpStream,
    started: Instant,
    deadline: Duration,
    idle: Duration,
}
impl Transport<'_> {
    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_sub(self.started.elapsed())
            .filter(|d| !d.is_zero())
            .map(|d| d.min(self.idle))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "I/O deadline exceeded"))
    }
}
impl Read for Transport<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        let count = self.stream.read(bytes)?;
        self.remaining()?;
        Ok(count)
    }
}
impl Write for Transport<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        let count = self.stream.write(bytes)?;
        self.remaining()?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

pub(super) fn serve(
    mut stream: TcpStream,
    accepted: Instant,
    app: &App,
    shutdown: &crate::ShutdownHandle,
) -> bool {
    let config = app.config();
    if stream.set_nonblocking(false).is_err() {
        return false;
    }
    let mut completed = false;
    for index in 0..config.max_requests_per_connection {
        if shutdown.is_requested() && index > 0 {
            break;
        }
        let started = if index == 0 { accepted } else { Instant::now() };
        let mut keep = false;
        let parsed = read_request(
            &mut Transport {
                stream: &mut stream,
                started,
                deadline: config.request_deadline,
                idle: config.read_timeout,
            },
            config,
        );
        let (method, response) = match parsed {
            Ok(request) => {
                keep = config.keep_alive
                    && index + 1 < config.max_requests_per_connection
                    && !shutdown.is_requested()
                    && !request
                        .headers()
                        .get_all("connection")
                        .any(|v| v.split(',').any(|t| t.trim().eq_ignore_ascii_case("close")));
                let method = request.method().clone();
                let response = match catch_unwind(AssertUnwindSafe(|| app.handle(request))) {
                    Ok(Ok(response)) => response,
                    Ok(Err(Error::Input(error))) => error.response(),
                    Ok(Err(Error::Http(HttpError::InvalidUtf8(_)))) => {
                        Response::text("Bad Request").status(400)
                    }
                    Ok(Err(_)) | Err(_) => Response::text("Internal Server Error").status(500),
                };
                (method, response)
            }
            Err(ProtocolError::Io(_)) => return completed, // Disconnected, truncated or timed-out input: close.
            Err(error) => {
                // Method may be unknown for malformed input; empty response is safe for HEAD too.
                (
                    Method::new("GET").expect("static method"),
                    Response::empty().status(error.status_code()),
                )
            }
        };
        let mut writer = Transport {
            stream: &mut stream,
            started: Instant::now(),
            deadline: config.write_timeout,
            idle: config.write_timeout,
        };
        // A failed/partial response cannot be retried on this connection.
        if write_response_connection(&mut writer, &response, &method, keep).is_err() {
            return false;
        }
        completed = true;
        if !keep {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    completed
}
