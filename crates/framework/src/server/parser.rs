use super::{validation::validate_request_headers, ProtocolError as E};
use crate::{Headers, Method, Request, ServerConfig, Validate};
use std::io::Read;

/// Parse one request without reading beyond its Content-Length.
/// The caller must enforce deadlines on the Read implementation.
pub fn read_request<R: Read>(reader: &mut R, config: &ServerConfig) -> Result<Request, E> {
    config.validate().map_err(E::InvalidConfiguration)?;
    let mut bytes = Vec::new();
    // Bytewise bounded header read avoids consuming the next message.
    loop {
        if bytes.len() >= config.max_header_bytes {
            return Err(E::HeaderLimit);
        }
        let mut byte = [0];
        reader.read_exact(&mut byte)?;
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| E::Malformed)?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let line = lines.next().ok_or(E::Malformed)?;
    let fields: Vec<_> = line.split(' ').collect();
    if fields.len() != 3 || fields.iter().any(|s| s.is_empty()) {
        return Err(E::Malformed);
    }
    let method = Method::new(fields[0]).map_err(|_| E::Malformed)?;
    if fields[2] != "HTTP/1.1" {
        return Err(E::UnsupportedVersion);
    }
    // Check the target before allocating or waiting for body bytes.
    Request::new(method.clone(), fields[1], Headers::new(), Vec::new())
        .map_err(|_| E::Malformed)?;
    let mut headers = Headers::new();
    for (index, line) in lines.enumerate() {
        if index >= config.max_headers {
            return Err(E::HeaderLimit);
        }
        let (name, value) = line.split_once(':').ok_or(E::Malformed)?;
        headers.append(name, value).map_err(|_| E::Malformed)?;
    }

    let framing = validate_request_headers(&headers, config)?;
    if framing.chunked {
        let body = read_chunks(reader, config)?;
        return Request::new(method, fields[1], headers, body).map_err(|_| E::Malformed);
    }

    let length = framing.content_length.unwrap_or(0);
    let mut body = Vec::new();
    // Grow as input arrives instead of allocating the entire declared size upfront.
    let mut chunk = [0u8; 8192];
    while body.len() < length {
        let count = chunk.len().min(length - body.len());
        reader.read_exact(&mut chunk[..count])?;
        body.extend_from_slice(&chunk[..count]);
    }
    Request::new(method, fields[1], headers, body).map_err(|_| E::Malformed)
}

fn line<R: Read>(reader: &mut R, budget: &mut usize) -> Result<Vec<u8>, E> {
    let mut out = Vec::new();
    loop {
        if *budget == 0 {
            return Err(E::HeaderLimit);
        }
        *budget -= 1;
        let mut byte = [0];
        reader.read_exact(&mut byte)?;
        out.push(byte[0]);
        if out.ends_with(b"\r\n") {
            out.truncate(out.len() - 2);
            return Ok(out);
        }
    }
}

fn read_chunks<R: Read>(reader: &mut R, config: &ServerConfig) -> Result<Vec<u8>, E> {
    let mut body = Vec::new();
    let mut budget = config.max_header_bytes;
    loop {
        let size = line(reader, &mut budget)?;
        // Extensions and trailers are deliberately unsupported in this first subset.
        if size.is_empty() || !size.iter().all(u8::is_ascii_hexdigit) {
            return Err(E::Malformed);
        }
        let size = usize::from_str_radix(std::str::from_utf8(&size).map_err(|_| E::Malformed)?, 16)
            .map_err(|_| E::BodyLimit)?;
        if size == 0 {
            if !line(reader, &mut budget)?.is_empty() {
                return Err(E::Malformed);
            }
            return Ok(body);
        }
        if size > config.max_body_bytes.saturating_sub(body.len()) {
            return Err(E::BodyLimit);
        }
        let mut remaining = size;
        let mut buf = [0; 8192];
        while remaining > 0 {
            let n = remaining.min(buf.len());
            reader.read_exact(&mut buf[..n])?;
            body.extend_from_slice(&buf[..n]);
            remaining -= n;
        }
        let mut crlf = [0; 2];
        reader.read_exact(&mut crlf)?;
        if crlf != *b"\r\n" {
            return Err(E::Malformed);
        }
    }
}
