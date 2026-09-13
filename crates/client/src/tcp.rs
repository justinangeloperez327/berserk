use crate::{ClientError, ErrorKind, Header, HttpClient, Method, Request, Response, Result};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpClientConfig {
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_request_header_bytes: usize,
    pub max_request_headers: usize,
    pub max_request_body_bytes: usize,
    pub max_response_header_bytes: usize,
    pub max_response_headers: usize,
    pub max_response_body_bytes: usize,
}
impl Default for TcpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(15),
            write_timeout: Duration::from_secs(15),
            max_request_header_bytes: 32 * 1024,
            max_request_headers: 100,
            max_request_body_bytes: 8 * 1024 * 1024,
            max_response_header_bytes: 32 * 1024,
            max_response_headers: 100,
            max_response_body_bytes: 8 * 1024 * 1024,
        }
    }
}

pub struct TcpHttpClient {
    config: TcpClientConfig,
}
impl TcpHttpClient {
    pub fn new(config: TcpClientConfig) -> Result<Self> {
        if config.connect_timeout.is_zero()
            || config.read_timeout.is_zero()
            || config.write_timeout.is_zero()
            || config.max_request_header_bytes == 0
            || config.max_request_headers == 0
            || config.max_request_body_bytes == 0
            || config.max_response_header_bytes == 0
            || config.max_response_headers == 0
            || config.max_response_body_bytes == 0
        {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "HTTP client limits and timeouts must be greater than zero",
            ));
        }
        Ok(Self { config })
    }
}
impl Default for TcpHttpClient {
    fn default() -> Self {
        Self {
            config: TcpClientConfig::default(),
        }
    }
}

impl HttpClient for TcpHttpClient {
    fn send(&self, request: Request) -> Result<Response> {
        if request.url().scheme() != "http" {
            return Err(ClientError::new(ErrorKind::UnsupportedScheme, "built-in TCP transport supports plaintext http only; configure a TLS-capable adapter for https"));
        }
        if request.headers().len() > self.config.max_request_headers {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "request has too many headers",
            ));
        }
        let fixed_bytes = request
            .method()
            .as_str()
            .len()
            .checked_add(request.url().path_and_query().len())
            .and_then(|total| total.checked_add(request.url().authority().len()))
            .and_then(|total| total.checked_add(96))
            .ok_or_else(|| {
                ClientError::new(
                    ErrorKind::InvalidRequest,
                    "request header byte count overflow",
                )
            })?;
        let header_bytes = request
            .headers()
            .iter()
            .try_fold(fixed_bytes, |total, header| {
                total
                    .checked_add(header.name().len())?
                    .checked_add(header.value().len())?
                    .checked_add(4)
            })
            .ok_or_else(|| {
                ClientError::new(
                    ErrorKind::InvalidRequest,
                    "request header byte count overflow",
                )
            })?;
        if header_bytes > self.config.max_request_header_bytes {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "request headers exceed configured byte limit",
            ));
        }
        if request.body_bytes().len() > self.config.max_request_body_bytes {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                "request body exceeds configured byte limit",
            ));
        }
        validate_transport_headers(request.headers())?;
        let addresses: Vec<_> = (request.url().host(), request.url().port())
            .to_socket_addrs()
            .map_err(|error| ClientError::new(ErrorKind::Dns, error.to_string()))?
            .collect();
        if addresses.is_empty() {
            return Err(ClientError::new(
                ErrorKind::Dns,
                "URL host resolved to no addresses",
            ));
        }
        let mut last_error = None;
        let mut connected = None;
        for address in addresses {
            match TcpStream::connect_timeout(&address, self.config.connect_timeout) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }
        let mut stream = connected.ok_or_else(|| {
            map_io(
                ErrorKind::Connect,
                last_error.unwrap_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no address available")
                }),
            )
        })?;
        stream
            .set_read_timeout(Some(self.config.read_timeout))
            .map_err(|error| map_io(ErrorKind::Io, error))?;
        stream
            .set_write_timeout(Some(self.config.write_timeout))
            .map_err(|error| map_io(ErrorKind::Io, error))?;
        write_request(&mut stream, &request)?;
        read_response(stream, request.method(), self.config)
    }
}

fn validate_transport_headers(headers: &[Header]) -> Result<()> {
    for header in headers {
        if matches!(
            header.name(),
            "host" | "connection" | "content-length" | "transfer-encoding"
        ) {
            return Err(ClientError::new(
                ErrorKind::InvalidRequest,
                format!("{} is controlled by the HTTP transport", header.name()),
            ));
        }
    }
    Ok(())
}
fn write_request(stream: &mut TcpStream, request: &Request) -> Result<()> {
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        request.method().as_str(),
        request.url().path_and_query(),
        request.url().authority(),
        request.body_bytes().len()
    );
    for header in request.headers() {
        head.push_str(header.name());
        head.push_str(": ");
        head.push_str(header.value());
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|error| map_io(ErrorKind::Write, error))?;
    stream
        .write_all(request.body_bytes())
        .map_err(|error| map_io(ErrorKind::Write, error))?;
    stream
        .flush()
        .map_err(|error| map_io(ErrorKind::Write, error))
}
fn read_response(stream: TcpStream, method: Method, config: TcpClientConfig) -> Result<Response> {
    let mut reader = BufReader::new(stream);
    let mut used = 0_usize;
    let status_line = read_line(&mut reader, &mut used, config.max_response_header_bytes)?;
    let mut parts = status_line
        .trim_end_matches(|character| character == '\r' || character == '\n')
        .splitn(3, ' ');
    let version = parts.next().unwrap_or("");
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response has an unsupported HTTP version",
        ));
    }
    let status: u16 = parts
        .next()
        .ok_or_else(|| {
            ClientError::new(ErrorKind::MalformedResponse, "response status is missing")
        })?
        .parse()
        .map_err(|_| {
            ClientError::new(ErrorKind::MalformedResponse, "response status is invalid")
        })?;
    if !(100..=599).contains(&status) {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response status is outside the HTTP range",
        ));
    }
    if status < 200 {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "informational and upgrade responses are unsupported",
        ));
    }
    let mut headers = Vec::new();
    loop {
        let line = read_line(&mut reader, &mut used, config.max_response_header_bytes)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if headers.len() >= config.max_response_headers {
            return Err(ClientError::new(
                ErrorKind::ResponseTooLarge,
                "response has too many headers",
            ));
        }
        let line = line.trim_end_matches(|character| character == '\r' || character == '\n');
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(ClientError::new(
                ErrorKind::MalformedResponse,
                "folded response headers are unsupported",
            ));
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            ClientError::new(ErrorKind::MalformedResponse, "response header is malformed")
        })?;
        headers.push(Header::new(name, value.trim_start()).map_err(|_| {
            ClientError::new(ErrorKind::MalformedResponse, "response header is invalid")
        })?);
    }
    let bodyless =
        method == Method::Head || (100..200).contains(&status) || matches!(status, 204 | 205 | 304);
    let body = if bodyless {
        Vec::new()
    } else if transfer_chunked(&headers)? {
        read_chunked(&mut reader, config)?
    } else if let Some(length) = content_length(&headers)? {
        read_exact_bounded(&mut reader, length, config.max_response_body_bytes)?
    } else {
        read_to_end_bounded(&mut reader, config.max_response_body_bytes)?
    };
    Response::new(status, headers, body)
}
fn read_line(reader: &mut impl BufRead, used: &mut usize, limit: usize) -> Result<String> {
    let mut bytes = Vec::new();
    let read = reader
        .read_until(b'\n', &mut bytes)
        .map_err(|error| map_io(ErrorKind::Io, error))?;
    if read == 0 {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response ended before its headers completed",
        ));
    }
    if !bytes.ends_with(b"\n") {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response line is missing its newline terminator",
        ));
    }
    *used = used.checked_add(read).ok_or_else(|| {
        ClientError::new(
            ErrorKind::ResponseTooLarge,
            "response header byte count overflow",
        )
    })?;
    if *used > limit {
        return Err(ClientError::new(
            ErrorKind::ResponseTooLarge,
            "response headers exceed configured byte limit",
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        ClientError::new(
            ErrorKind::MalformedResponse,
            "response headers are not valid UTF-8",
        )
    })
}
fn content_length(headers: &[Header]) -> Result<Option<usize>> {
    let values: Vec<_> = headers
        .iter()
        .filter(|header| header.name() == "content-length")
        .map(Header::value)
        .collect();
    if values.is_empty() {
        return Ok(None);
    }
    if values.iter().any(|value| *value != values[0]) {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response has conflicting content lengths",
        ));
    }
    values[0].parse().map(Some).map_err(|_| {
        ClientError::new(
            ErrorKind::MalformedResponse,
            "response content length is invalid",
        )
    })
}
fn transfer_chunked(headers: &[Header]) -> Result<bool> {
    let values: Vec<_> = headers
        .iter()
        .filter(|header| header.name() == "transfer-encoding")
        .map(|header| header.value().to_ascii_lowercase())
        .collect();
    if values.is_empty() {
        return Ok(false);
    }
    if values.len() != 1 || values[0].trim() != "chunked" {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "unsupported response transfer encoding",
        ));
    }
    if headers
        .iter()
        .any(|header| header.name() == "content-length")
    {
        return Err(ClientError::new(
            ErrorKind::MalformedResponse,
            "response combines transfer encoding with content length",
        ));
    }
    Ok(true)
}
fn read_exact_bounded(reader: &mut impl Read, length: usize, limit: usize) -> Result<Vec<u8>> {
    if length > limit {
        return Err(ClientError::new(
            ErrorKind::ResponseTooLarge,
            "response body exceeds configured byte limit",
        ));
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| map_io(ErrorKind::Io, error))?;
    Ok(body)
}
fn read_to_end_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| map_io(ErrorKind::Io, error))?;
        if read == 0 {
            break;
        }
        if body
            .len()
            .checked_add(read)
            .is_none_or(|length| length > limit)
        {
            return Err(ClientError::new(
                ErrorKind::ResponseTooLarge,
                "response body exceeds configured byte limit",
            ));
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(body)
}
fn read_chunked(reader: &mut impl BufRead, config: TcpClientConfig) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let mut trailer_bytes = 0;
    let mut trailer_count = 0;
    loop {
        let line = read_line(reader, &mut trailer_bytes, config.max_response_header_bytes)?;
        let size_text = line
            .trim_end_matches(|character| character == '\r' || character == '\n')
            .split(';')
            .next()
            .unwrap_or("");
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| ClientError::new(ErrorKind::MalformedResponse, "chunk size is invalid"))?;
        if size == 0 {
            loop {
                let trailer =
                    read_line(reader, &mut trailer_bytes, config.max_response_header_bytes)?;
                if trailer == "\r\n" || trailer == "\n" {
                    return Ok(body);
                }
                trailer_count += 1;
                if trailer_count > config.max_response_headers {
                    return Err(ClientError::new(
                        ErrorKind::ResponseTooLarge,
                        "response has too many trailers",
                    ));
                }
                if !trailer.contains(':') {
                    return Err(ClientError::new(
                        ErrorKind::MalformedResponse,
                        "response trailer is malformed",
                    ));
                }
            }
        }
        if body
            .len()
            .checked_add(size)
            .is_none_or(|length| length > config.max_response_body_bytes)
        {
            return Err(ClientError::new(
                ErrorKind::ResponseTooLarge,
                "response body exceeds configured byte limit",
            ));
        }
        let start = body.len();
        body.resize(start + size, 0);
        reader
            .read_exact(&mut body[start..])
            .map_err(|error| map_io(ErrorKind::Io, error))?;
        let mut ending = [0_u8; 2];
        reader
            .read_exact(&mut ending)
            .map_err(|error| map_io(ErrorKind::Io, error))?;
        if ending != *b"\r\n" {
            return Err(ClientError::new(
                ErrorKind::MalformedResponse,
                "chunk is missing its CRLF terminator",
            ));
        }
    }
}
fn map_io(default: ErrorKind, error: std::io::Error) -> ClientError {
    let kind = if matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    ) {
        ErrorKind::Timeout
    } else {
        default
    };
    ClientError::new(kind, error.to_string())
}
