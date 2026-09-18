use super::HttpError;

pub(super) fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn validate(name: &str, value: &str) -> Result<(), HttpError> {
    if !token(name) {
        return Err(HttpError::InvalidHeaderName);
    }

    // Deliberate ASCII text subset; no CR/LF, DEL, NUL or obs-text.
    if !value.bytes().all(|b| b == b'\t' || (32..=126).contains(&b)) {
        return Err(HttpError::InvalidHeaderValue);
    }

    Ok(())
}

/// Ordered, repeated headers with case-insensitive lookup.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Headers(Vec<(String, String)>);

impl std::fmt::Debug for Headers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Headers")
            .field(
                "names",
                &self.0.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            )
            .field("count", &self.0.len())
            .finish()
    }
}

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, name: &str, value: &str) -> Result<(), HttpError> {
        validate(name, value)?;

        self.0.push((
            name.to_ascii_lowercase(),
            value.trim_matches([' ', '\t']).to_owned(),
        ));

        Ok(())
    }

    /// Validation occurs before replacement so failure leaves headers intact.
    pub fn insert(&mut self, name: &str, value: &str) -> Result<(), HttpError> {
        validate(name, value)?;

        self.0.retain(|(key, _)| !key.eq_ignore_ascii_case(name));

        self.0.push((
            name.to_ascii_lowercase(),
            value.trim_matches([' ', '\t']).to_owned(),
        ));

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Remove every value for a header name, ignoring ASCII case.
    pub fn remove(&mut self, name: &str) {
        self.0.retain(|(key, _)| !key.eq_ignore_ascii_case(name));
    }

    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.0
            .iter()
            .filter(move |(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
