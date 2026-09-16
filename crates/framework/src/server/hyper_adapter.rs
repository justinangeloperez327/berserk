use super::{validation::validate_request_headers, ProtocolError};
use bytes::Bytes;
use http_body_util::{BodyExt, LengthLimitError, Limited};
use std::error::Error as StdError;

use crate::{Headers, Method, Request, ServerConfig, Validate};

type BoxError = Box<dyn StdError + Send + Sync>;

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

    let collected = Limited::new(body, config.max_body_bytes)
        .collect()
        .await
        .map_err(|error| {
            if error.downcast_ref::<LengthLimitError>().is_some() {
                ProtocolError::BodyLimit
            } else {
                ProtocolError::Malformed
            }
        })?;
    if collected.trailers().is_some() {
        return Err(ProtocolError::Malformed);
    }
    let body = collected.to_bytes();

    match framing.content_length {
        Some(length) if body.len() != length => return Err(ProtocolError::Malformed),
        None if !framing.chunked && !body.is_empty() => return Err(ProtocolError::Malformed),
        _ => {}
    }

    Request::new(method, target, headers, body.to_vec()).map_err(|_| ProtocolError::Malformed)
}
