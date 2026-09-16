use super::RouteError;
use crate::{Headers, Method, Request};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
enum Segment {
    Static(String),
    Param(String),
}

#[derive(Debug)]
pub(super) struct Pattern {
    segments: Vec<Segment>,
    source: String,
}

impl Pattern {
    pub(super) fn parse(path: &str) -> Result<Self, RouteError> {
        if !path.starts_with('/') || path.contains(['?', '#', '*']) {
            return Err(RouteError::InvalidPattern);
        }
        let mut segments = Vec::new();
        let mut names = std::collections::HashSet::new();
        let mut probe = Vec::new();
        for part in path.split('/') {
            if let Some(name) = part.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                let mut chars = name.bytes();
                let first = chars.next().ok_or(RouteError::InvalidPattern)?;
                if !(first.is_ascii_alphabetic() || first == b'_')
                    || !chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
                {
                    return Err(RouteError::InvalidPattern);
                }
                if !names.insert(name.to_owned()) {
                    return Err(RouteError::DuplicateParameter);
                }
                segments.push(Segment::Param(name.to_owned()));
                probe.push("x");
            } else {
                if part.contains(['{', '}']) {
                    return Err(RouteError::InvalidPattern);
                }
                segments.push(Segment::Static(part.to_owned()));
                probe.push(part);
            }
        }
        Request::new(
            Method::new("GET").expect("valid static method"),
            probe.join("/"),
            Headers::new(),
            Vec::new(),
        )
        .map_err(|_| RouteError::InvalidPattern)?;
        Ok(Self {
            segments,
            source: path.to_owned(),
        })
    }

    pub(super) fn prefixed(&self, prefix: &str) -> Result<Self, RouteError> {
        if prefix.is_empty() || prefix == "/" {
            return Self::parse(&self.source);
        }
        if prefix.ends_with('/') {
            return Err(RouteError::InvalidPattern);
        }
        Self::parse(&format!("{prefix}{}", self.source))
    }

    pub(super) fn equivalent(&self, other: &Self) -> bool {
        self.segments.len() == other.segments.len()
            && self
                .segments
                .iter()
                .zip(&other.segments)
                .all(|(a, b)| match (a, b) {
                    (Segment::Static(a), Segment::Static(b)) => a == b,
                    (Segment::Param(_), Segment::Param(_)) => true,
                    _ => false,
                })
    }

    pub(super) fn same_template(&self, other: &Self) -> bool {
        self.source == other.source
    }

    pub(super) fn source(&self) -> &str {
        &self.source
    }

    pub(super) fn parameter_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| matches!(segment, Segment::Param(_)))
            .count()
    }

    pub(super) fn captures(&self, path: &str) -> Option<HashMap<String, String>> {
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != self.segments.len() {
            return None;
        }
        let mut params = HashMap::new();
        for (segment, value) in self.segments.iter().zip(parts) {
            match segment {
                Segment::Static(expected) if expected == value => {}
                Segment::Param(name) if !value.is_empty() => {
                    params.insert(name.clone(), decode_path_segment(value)?);
                }
                _ => return None,
            }
        }
        Some(params)
    }

    pub(super) fn build(&self, params: &BTreeMap<String, String>) -> Result<String, RouteError> {
        let mut used = std::collections::BTreeSet::new();
        let mut output = String::new();
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                output.push('/');
            }
            match segment {
                Segment::Static(value) => output.push_str(value),
                Segment::Param(name) => {
                    let value = params
                        .get(name)
                        .ok_or_else(|| RouteError::MissingRouteParameter(name.clone()))?;
                    used.insert(name.as_str());
                    encode_path_segment(value, &mut output);
                }
            }
        }
        if let Some(name) = params.keys().find(|name| !used.contains(name.as_str())) {
            return Err(RouteError::UnknownRouteParameter(name.clone()));
        }
        Ok(output)
    }

    pub(super) fn specificity(&self) -> Vec<bool> {
        self.segments
            .iter()
            .map(|s| matches!(s, Segment::Static(_)))
            .collect()
    }
}

fn encode_path_segment(value: &str, output: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(byte as char);
        } else {
            output.push('%');
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
}

fn decode_path_segment(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
