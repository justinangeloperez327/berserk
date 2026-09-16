use super::ProtocolError as E;
use crate::{Headers, ServerConfig};
use std::net::Ipv6Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RequestFraming {
    pub(super) content_length: Option<usize>,
    pub(super) chunked: bool,
}

fn valid_host(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let Some((address, suffix)) = rest.split_once(']') else {
            return false;
        };
        if address.parse::<Ipv6Addr>().is_err() {
            return false;
        }
        if suffix.is_empty() {
            return true;
        }
        let Some(port) = suffix.strip_prefix(':') else {
            return false;
        };
        (address, Some(port))
    } else {
        let (host, port) = value
            .split_once(':')
            .map_or((value, None), |(host, port)| (host, Some(port)));
        if !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
        {
            return false;
        }
        (host, port)
    };

    !host.is_empty()
        && port.is_none_or(|port| {
            !port.is_empty()
                && port.bytes().all(|byte| byte.is_ascii_digit())
                && port.parse::<u16>().is_ok()
        })
}

pub(super) fn validate_request_headers(
    headers: &Headers,
    config: &ServerConfig,
) -> Result<RequestFraming, E> {
    let hosts: Vec<_> = headers.get_all("host").collect();
    if hosts.len() != 1 || !valid_host(hosts[0]) {
        return Err(E::Malformed);
    }

    let lengths: Vec<_> = headers.get_all("content-length").collect();
    if lengths.len() > 1 {
        return Err(E::Malformed);
    }

    let encodings: Vec<_> = headers.get_all("transfer-encoding").collect();
    let chunked = !encodings.is_empty();
    if chunked && !lengths.is_empty() {
        return Err(E::Malformed);
    }
    if chunked && (encodings.len() != 1 || !encodings[0].eq_ignore_ascii_case("chunked")) {
        return Err(E::UnsupportedTransferEncoding);
    }
    if headers.get("expect").is_some() {
        return Err(E::UnsupportedExpectation);
    }

    let content_length = if let Some(value) = lengths.first() {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(E::Malformed);
        }
        let length = value.parse::<usize>().map_err(|_| E::BodyLimit)?;
        if length > config.max_body_bytes {
            return Err(E::BodyLimit);
        }
        Some(length)
    } else {
        None
    };

    Ok(RequestFraming {
        content_length,
        chunked,
    })
}
