use berserk::{App, Request, Response, Result};

fn build_app() -> Result<App> {
    let mut app = App::new();
    {
        let mut route = app.route();
        route.get("/", || Response::text("Hello, world!"))?;
        route.get("/health", || Response::text("OK"))?;
        route.get("/users/{id}", show_user)?;
        route.post("/echo", |req: Request| Response::bytes(req.body().to_vec()))?;
        route.post("/text", echo_text)?;
        route.delete("/items/{id}", |_id: String| Response::empty().status(204))?;
    }
    Ok(app)
}

fn show_user(id: u64) -> Response {
    Response::text(format!("User {id}"))
}

fn echo_text(request: Request) -> Result<Response> {
    Ok(Response::text(request.text()?))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3000".to_owned());
    let server = build_app()?.bind(address)?;
    println!("Listening on http://{}", server.local_addr()?);
    server.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{SocketAddr, TcpStream},
        time::Duration,
    };

    fn exchange(address: SocketAddr, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
        let mut output = Vec::new();
        stream.read_to_end(&mut output).unwrap();
        output
    }

    #[test]
    fn consumer_http_contract() {
        let server = build_app().unwrap().bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        struct Cleanup(berserk::ShutdownHandle);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let cleanup = Cleanup(server.shutdown_handle());
        let worker = std::thread::spawn(move || server.run());
        for (method, path, input, status, expected) in [
            ("GET", "/", &b""[..], 200, &b"Hello, world!"[..]),
            ("GET", "/health", &b""[..], 200, &b"OK"[..]),
            ("GET", "/users/42", &b""[..], 200, &b"User 42"[..]),
            ("POST", "/echo", &b"\x00\xff"[..], 200, &b"\x00\xff"[..]),
            ("POST", "/text", &b"text"[..], 200, &b"text"[..]),
            ("POST", "/text", &b"\xff"[..], 400, &b"Bad Request"[..]),
            ("DELETE", "/items/1", &b""[..], 204, &b""[..]),
            ("GET", "/missing", &b""[..], 404, &b"Not Found"[..]),
            ("POST", "/health", &b""[..], 405, &b"Method Not Allowed"[..]),
            ("HEAD", "/health", &b""[..], 200, &b""[..]),
        ] {
            let bytes = exchange(address, method, path, input);
            let split = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            let headers = std::str::from_utf8(&bytes[..split]).unwrap();
            assert!(
                headers.starts_with(&format!("HTTP/1.1 {status} ")),
                "{method} {path}"
            );
            assert_eq!(&bytes[split + 4..], expected);
            if status == 405 {
                assert!(headers.contains("allow: GET, HEAD"));
            }
            if method == "HEAD" {
                assert!(headers.contains("content-length: 2"));
            }
            if status == 204 {
                assert!(!headers.contains("content-length:"));
            }
        }
        drop(cleanup);
        worker.join().unwrap().unwrap();
    }
}
