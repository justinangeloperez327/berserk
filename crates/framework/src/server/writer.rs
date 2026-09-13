use super::ProtocolError;
use crate::{Method, Response};
use std::io::{Read, Write};

/// Validate first, then encode one final HTTP/1.1 response. Caller closes the connection.
/// May leave a partial response on I/O failure; never retry it on that connection.
pub fn write_response<W: Write>(
    writer: &mut W,
    response: &Response,
    request_method: &Method,
) -> Result<(), ProtocolError> {
    write_response_connection(writer, response, request_method, false)
}
pub fn write_response_connection<W: Write>(
    writer: &mut W,
    response: &Response,
    request_method: &Method,
    keep_alive: bool,
) -> Result<(), ProtocolError> {
    response
        .validate()
        .map_err(ProtocolError::InvalidResponse)?;
    let code = response.status_code();
    let head = request_method.as_str() == "HEAD";
    let mut source = if !head {
        match &response.stream {
            Some(stream) => Some(
                stream
                    .0
                    .lock()
                    .map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::Other, "stream lock poisoned")
                    })?
                    .take()
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::Other, "stream already consumed")
                    })?,
            ),
            None => None,
        }
    } else {
        None
    };
    let reason = match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        205 => "Reset Content",
        304 => "Not Modified",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Content Too Large",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "",
    };
    let mut header = format!("HTTP/1.1 {code} {reason}\r\n");
    for (name, value) in response.headers().iter() {
        header.push_str(name);
        header.push_str(": ");
        header.push_str(value);
        header.push_str("\r\n");
    }
    // Omit length for 204 and 304; no selected representation size is known for 304.
    if response.stream.is_some() {
        if !head {
            header.push_str("transfer-encoding: chunked\r\n");
        }
    } else if !matches!(code, 204 | 304) {
        let length = if code == 205 {
            0
        } else if head {
            response.representation_length()
        } else {
            response.body().len()
        };
        header.push_str(&format!("content-length: {length}\r\n"));
    }
    header.push_str(if keep_alive {
        "connection: keep-alive\r\n\r\n"
    } else {
        "connection: close\r\n\r\n"
    });
    writer.write_all(header.as_bytes())?;
    if let Some(source) = source.as_mut() {
        let mut buf = [0; 8192];
        loop {
            let n = match source.read(&mut buf) {
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                other => other?,
            };
            if n == 0 {
                break;
            }
            write!(writer, "{n:x}\r\n")?;
            writer.write_all(&buf[..n])?;
            writer.write_all(b"\r\n")?;
        }
        writer.write_all(b"0\r\n\r\n")?;
    } else if !head && !matches!(code, 204 | 205 | 304) {
        writer.write_all(response.body())?;
    }
    writer.flush()?;
    Ok(())
}
