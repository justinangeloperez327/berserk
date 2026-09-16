use crate::{Json, Request, Response, Result};
use std::ops::Deref;

pub use framework_validation::{FieldError, ValidateInput, ValidationErrors};

/// A typed request body that has already been decoded, sanitized, and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validated<T>(T);

impl<T> Validated<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(value)
    }

    pub fn get_ref(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> AsRef<T> for Validated<T> {
    fn as_ref(&self) -> &T {
        self.get_ref()
    }
}

impl<T> Deref for Validated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

#[derive(Debug)]
pub enum InputError {
    Json(crate::json::JsonError),
    ContentType,
    Query,
    Fields(ValidationErrors),
}
impl InputError {
    pub fn response(&self) -> Response {
        let (code, message, fields) = match self {
            Self::Json(_) => (400, "Invalid JSON", Vec::new()),
            Self::Query => (400, "Invalid query string", Vec::new()),
            Self::ContentType => (415, "Expected application/json", Vec::new()),
            Self::Fields(errors) => (
                422,
                "Validation failed",
                errors
                    .0
                    .iter()
                    .map(|e| {
                        Json::Object(
                            [
                                ("field".into(), Json::String(e.field.clone())),
                                ("code".into(), Json::String(e.code.clone())),
                                ("message".into(), Json::String(e.message.clone())),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        };
        let value = Json::Object(
            [
                ("message".into(), Json::String(message.into())),
                ("errors".into(), Json::Array(fields)),
            ]
            .into_iter()
            .collect(),
        );
        Response::json(&value)
            .expect("bounded error JSON")
            .status(code)
    }
}
impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("request input rejected")
    }
}
impl std::error::Error for InputError {}
/// Explicit conversion avoids reflection; implement per input type.
pub trait FromJson: Sized {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors>;
}
impl Request {
    pub fn json(&self) -> Result<Json> {
        let types: Vec<_> = self.headers().get_all("content-type").collect();
        if types.len() != 1
            || !types[0]
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("application/json")
        {
            return Err(InputError::ContentType.into());
        }
        Json::parse(self.body()).map_err(|e| InputError::Json(e).into())
    }
    pub fn validated<T: FromJson + ValidateInput>(&self) -> Result<T> {
        let value = self.json()?;
        let mut typed = T::from_json(&value).map_err(InputError::Fields)?;
        typed.sanitize();
        typed.validate().map_err(InputError::Fields)?;
        Ok(typed)
    }
    /// Form-style query decoding: repeated keys retained, '+' becomes space.
    pub fn query(&self) -> Result<Vec<(String, String)>> {
        let text = self.query_string().unwrap_or("");
        if text.is_empty() {
            return Ok(Vec::new());
        }
        text.split('&')
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                Ok((decode(k)?, decode(v)?))
            })
            .collect()
    }
}
fn decode(s: &str) -> Result<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= b.len() {
                    return Err(InputError::Query.into());
                }
                let a = (b[i + 1] as char).to_digit(16).ok_or(InputError::Query)?;
                let c = (b[i + 2] as char).to_digit(16).ok_or(InputError::Query)?;
                out.push((a * 16 + c) as u8);
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| InputError::Query.into())
}
impl Response {
    pub fn json(value: &Json) -> Result<Self> {
        let text = value
            .encode()
            .map_err(|_| crate::ConfigError::new("json", "response nesting exceeds limit"))?;
        Self::text(text).header("content-type", "application/json")
    }
}
