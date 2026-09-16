use super::{
    error::application_error_response, validation::validate_request_headers, ProtocolError,
};
use bytes::Bytes;
use http_body_util::{
    channel::Channel, combinators::UnsyncBoxBody, BodyExt, Full, LengthLimitError, Limited,
};
use std::{
    convert::Infallible,
    error::Error as StdError,
    io::{self, Read},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::Arc,
};

use crate::{Headers, Method, Request, Response, ServerConfig, Validate};

type BoxError = Box<dyn StdError + Send + Sync>;
pub(super) type WireBody = UnsyncBoxBody<Bytes, BoxError>;

fn infallible_to_box(error: Infallible) -> BoxError {
    match error {}
}

fn full_body(bytes: Bytes) -> WireBody {
    Full::new(bytes).map_err(infallible_to_box).boxed_unsync()
}

fn checked_add(total: &mut usize, value: usize) -> Result<(), ProtocolError> {
    *total = total.checked_add(value).ok_or(ProtocolError::HeaderLimit)?;
    Ok(())
}

fn request_header_bytes(
    method: &http::Method,
    target: &str,
    headers: &http::HeaderMap,
) -> Result<usize, ProtocolError> {
    let mut bytes = 0usize;
    for value in [
        method.as_str().len(),
        1,
        target.len(),
        1,
        b"HTTP/1.1\r\n".len(),
    ] {
        checked_add(&mut bytes, value)?;
    }
    for (name, value) in headers {
        for value in [name.as_str().len(), 2, value.as_bytes().len(), 2] {
            checked_add(&mut bytes, value)?;
        }
    }
    checked_add(&mut bytes, 2)?;
    Ok(bytes)
}

fn body_timeout_error() -> ProtocolError {
    ProtocolError::Io(io::Error::new(
        io::ErrorKind::TimedOut,
        "request body deadline exceeded",
    ))
}

async fn collect_body<B>(body: B, config: &ServerConfig) -> Result<Vec<u8>, ProtocolError>
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError> + 'static,
{
    let body = Limited::new(body, config.max_body_bytes);
    tokio::pin!(body);
    let deadline = tokio::time::Instant::now() + config.request_deadline;
    let mut bytes = Vec::new();

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(body_timeout_error());
        }
        let wait = config.read_timeout.min(remaining);
        let frame = tokio::time::timeout(
            wait,
            std::future::poll_fn(|cx| hyper::body::Body::poll_frame(body.as_mut(), cx)),
        )
        .await
        .map_err(|_| body_timeout_error())?;
        let Some(frame) = frame else {
            break;
        };
        let frame = frame.map_err(|error| {
            if error.downcast_ref::<LengthLimitError>().is_some() {
                ProtocolError::BodyLimit
            } else {
                ProtocolError::Io(io::Error::other(error.to_string()))
            }
        })?;
        if let Some(data) = frame.data_ref() {
            bytes.extend_from_slice(data);
        } else {
            return Err(ProtocolError::Malformed);
        }
    }

    Ok(bytes)
}

pub(super) async fn into_berserk_request<B>(
    request: http::Request<B>,
    config: &ServerConfig,
) -> Result<Request, ProtocolError>
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError> + 'static,
{
    config
        .validate()
        .map_err(ProtocolError::InvalidConfiguration)?;

    let (parts, body) = request.into_parts();
    if parts.version != http::Version::HTTP_11 {
        return Err(ProtocolError::UnsupportedVersion);
    }
    if parts.uri.scheme().is_some() || parts.uri.authority().is_some() {
        return Err(ProtocolError::Malformed);
    }

    let target = parts
        .uri
        .path_and_query()
        .map(http::uri::PathAndQuery::as_str)
        .unwrap_or("/");
    if parts.headers.len() > config.max_headers
        || request_header_bytes(&parts.method, target, &parts.headers)? > config.max_header_bytes
    {
        return Err(ProtocolError::HeaderLimit);
    }

    let method = Method::new(parts.method.as_str()).map_err(|_| ProtocolError::Malformed)?;
    let mut headers = Headers::new();
    for (name, value) in &parts.headers {
        let value = value.to_str().map_err(|_| ProtocolError::Malformed)?;
        headers
            .append(name.as_str(), value)
            .map_err(|_| ProtocolError::Malformed)?;
    }
    let framing = validate_request_headers(&headers, config)?;
    let body = collect_body(body, config).await?;

    match framing.content_length {
        Some(length) if body.len() != length => return Err(ProtocolError::Malformed),
        None if !framing.chunked && !body.is_empty() => return Err(ProtocolError::Malformed),
        _ => {}
    }

    Request::new(method, target, headers, body).map_err(|_| ProtocolError::Malformed)
}

fn stream_body(stream: crate::http::StreamBody) -> Result<WireBody, ProtocolError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|error| {
        ProtocolError::Io(io::Error::other(format!(
            "stream response requires a Tokio runtime: {error}"
        )))
    })?;
    let producer_runtime = runtime.clone();
    let (mut sender, body) = Channel::<Bytes, io::Error>::new(4);

    runtime.spawn_blocking(move || {
        let mut reader = match stream.0.lock() {
            Ok(mut source) => match source.take() {
                Some(reader) => reader,
                None => {
                    sender.abort(io::Error::other("stream already consumed"));
                    return;
                }
            },
            Err(_) => {
                sender.abort(io::Error::other("stream lock poisoned"));
                return;
            }
        };

        let mut buffer = [0u8; 8192];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    sender.abort(error);
                    return;
                }
            };

            let bytes = Bytes::copy_from_slice(&buffer[..count]);
            if producer_runtime.block_on(sender.send_data(bytes)).is_err() {
                return;
            }
        }
    });

    Ok(body
        .map_err(|error| Box::new(error) as BoxError)
        .boxed_unsync())
}

pub(super) fn into_wire_response(
    mut response: Response,
    head: bool,
) -> Result<http::Response<WireBody>, ProtocolError> {
    response
        .validate()
        .map_err(ProtocolError::InvalidResponse)?;

    let status = http::StatusCode::from_u16(response.status_code()).map_err(|_| {
        ProtocolError::InvalidResponse(crate::HttpError::InvalidStatus(response.status_code()))
    })?;
    let streamed = response.stream.is_some();
    let representation_length = response.representation_length();

    let body = if head {
        full_body(Bytes::new())
    } else if let Some(stream) = response.stream.take() {
        stream_body(stream)?
    } else {
        full_body(Bytes::from(response.take_body()))
    };

    let mut wire = http::Response::new(body);
    *wire.status_mut() = status;
    for (name, value) in response.headers().iter() {
        let name = http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| ProtocolError::InvalidResponse(crate::HttpError::InvalidHeaderName))?;
        let value = http::HeaderValue::from_bytes(value.as_bytes())
            .map_err(|_| ProtocolError::InvalidResponse(crate::HttpError::InvalidHeaderValue))?;
        wire.headers_mut().append(name, value);
    }

    if !streamed && !matches!(response.status_code(), 204 | 304) {
        let length = representation_length.to_string();
        wire.headers_mut().insert(
            http::header::CONTENT_LENGTH,
            http::HeaderValue::from_bytes(length.as_bytes())
                .expect("usize is a valid header value"),
        );
    }

    Ok(wire)
}

fn fallback_wire_response(status: u16) -> http::Response<WireBody> {
    let mut response = http::Response::new(full_body(Bytes::new()));
    *response.status_mut() =
        http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
    response
}

pub(super) async fn dispatch<B>(
    app: Arc<crate::App>,
    request: http::Request<B>,
) -> Result<http::Response<WireBody>, ProtocolError>
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError> + 'static,
{
    let head = request.method() == http::Method::HEAD;
    let request = match into_berserk_request(request, app.config()).await {
        Ok(request) => request,
        Err(error @ ProtocolError::Io(_)) => return Err(error),
        Err(error) => return Ok(fallback_wire_response(error.status_code())),
    };

    let handled =
        tokio::task::spawn_blocking(move || catch_unwind(AssertUnwindSafe(|| app.handle(request))))
            .await;

    let response = match handled {
        Ok(Ok(Ok(response))) => response,
        Ok(Ok(Err(error))) => application_error_response(error),
        Ok(Err(_)) | Err(_) => Response::text("Internal Server Error").status(500),
    };

    Ok(match into_wire_response(response, head) {
        Ok(response) => response,
        Err(_) => fallback_wire_response(500),
    })
}
