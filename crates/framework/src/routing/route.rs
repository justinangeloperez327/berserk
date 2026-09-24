use super::RouteError;
use crate::{Headers, Method, Request};
use std::collections::BTreeMap;

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

    pub(super) fn matches(&self, path: &str) -> bool {
        let mut parts = path.split('/');
        for segment in &self.segments {
            let Some(value) = parts.next() else {
                return false;
            };
            match segment {
                Segment::Static(expected) if expected == value => {}
                Segment::Param(_) if !value.is_empty() => {}
                _ => return false,
            }
        }
        parts.next().is_none()
    }

    pub(super) fn captures(&self, path: &str) -> Option<Vec<(String, String)>> {
        let mut parts = path.split('/');
        let mut params = Vec::with_capacity(self.parameter_count());
        for segment in &self.segments {
            let value = parts.next()?;

            match segment {
                Segment::Static(expected) if expected == value => {}
                Segment::Param(name) if !value.is_empty() => {
                    params.push((name.clone(), value.to_owned()));
                }
                _ => return None,
            }
        }
        if parts.next().is_some() {
            return None;
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

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn allocation_free_match_probe_preserves_capture_semantics() {
        let pattern = Pattern::parse("/projects/{project}/tasks/{task}").unwrap();
        assert!(pattern.matches("/projects/7/tasks/11"));
        assert!(!pattern.matches("/projects/7/tasks"));
        assert!(!pattern.matches("/projects//tasks/11"));
        assert!(!pattern.matches("/projects/7/comments/11"));

        let captures = pattern.captures("/projects/7/tasks/11").unwrap();
        assert_eq!(
            captures,
            vec![
                ("project".to_owned(), "7".to_owned()),
                ("task".to_owned(), "11".to_owned()),
            ]
        );
    }

    #[test]
    fn root_and_trailing_slash_matching_remain_distinct() {
        assert!(Pattern::parse("/").unwrap().matches("/"));
        assert!(Pattern::parse("/users").unwrap().matches("/users"));
        assert!(!Pattern::parse("/users").unwrap().matches("/users/"));
        assert!(Pattern::parse("/users/").unwrap().matches("/users/"));
    }
}
