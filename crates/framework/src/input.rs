use crate::{Json, Request, Response, Result};
use std::ops::Deref;

pub use berserk_validation::{FieldError, ValidateInput, ValidationErrors};

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
    Object,
    Query,
    Fields(ValidationErrors),
}
impl InputError {
    pub fn response(&self) -> Response {
        let (code, message, fields) = match self {
            Self::Json(_) => (400, "Invalid JSON", Vec::new()),
            Self::Query => (400, "Invalid query string", Vec::new()),
            Self::ContentType => (415, "Expected application/json", Vec::new()),
            Self::Object => (400, "Expected a JSON object", Vec::new()),
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

impl FromJson for Json {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        Ok(value.clone())
    }
}

impl Request {
    /// Decodes JSON into an explicit input type. Use `validate` for the FormRequest lifecycle.
    pub fn json<T: FromJson>(&self) -> Result<T> {
        T::from_json(&self.json_value()?).map_err(|errors| InputError::Fields(errors).into())
    }

    /// Parses a JSON value without application-specific decoding or validation.
    pub fn json_value(&self) -> Result<Json> {
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
        let value = self.json_value()?;
        let mut typed = T::from_json(&value).map_err(InputError::Fields)?;
        typed.sanitize();
        typed.validate().map_err(InputError::Fields)?;
        Ok(typed)
    }
    /// Returns one decoded query value. Repeated values for this name are rejected.
    pub fn query(&self, name: &str) -> Result<Option<String>> {
        let mut value = None;
        for (key, item) in self.query_pairs()? {
            if key == name && value.replace(item).is_some() {
                return Err(InputError::Query.into());
            }
        }
        Ok(value)
    }

    /// JSON object fields take precedence over query values, including explicit JSON null.
    /// Query values are returned as JSON strings. This accessor does not validate input.
    pub fn input(&self, name: &str) -> Result<Option<Json>> {
        if !self.body().is_empty() {
            let Json::Object(mut fields) = self.json_value()? else {
                return Err(InputError::Object.into());
            };
            if let Some(value) = fields.remove(name) {
                return Ok(Some(value));
            }
        }
        Ok(self.query(name)?.map(Json::String))
    }

    /// Form-style query decoding: repeated keys retained, '+' becomes space.
    pub fn query_pairs(&self) -> Result<Vec<(String, String)>> {
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

pub type ValidationResult = std::result::Result<(), ValidationErrors>;

/// Input lifecycle: decode, sanitize, authorize, validate, then validate with request context.
pub trait FormRequest: FromJson {
    fn sanitize(&mut self) {}
    fn validate(&self) -> ValidationResult;
    fn validate_request(&self, _request: &Request) -> Result<()> {
        Ok(())
    }
    fn authorize(&self) -> bool {
        true
    }
    fn authorize_request(&self, _request: &Request) -> Result<()> {
        if self.authorize() {
            Ok(())
        } else {
            Err(crate::Error::forbidden())
        }
    }
}
impl Request {
    /// Decode and sanitize input, authorize it, then run field and request-aware validation.
    pub fn validate<T: FormRequest>(&self) -> Result<T> {
        self.form_request()
    }

    pub fn form_request<T: FormRequest>(&self) -> Result<T> {
        let mut input = T::from_json(&self.json_value()?).map_err(InputError::Fields)?;
        input.sanitize();
        input.authorize_request(self)?;
        input.validate().map_err(InputError::Fields)?;
        input.validate_request(self)?;
        Ok(input)
    }
    /// One positive decimal `page` value; invalid and duplicate values are rejected.
    pub fn page_number(&self) -> Result<u64> {
        let mut page = None;
        for (key, value) in self.query_pairs()? {
            if key == "page" {
                if page.is_some() || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(InputError::Query.into());
                }
                let number = value.parse::<u64>().map_err(|_| InputError::Query)?;
                if number == 0 {
                    return Err(InputError::Query.into());
                }
                page = Some(number);
            }
        }
        Ok(page.unwrap_or(1))
    }
}


#[cfg(test)]
mod form_request_tests {
    use super::*;

    struct DeniedInput;

    impl FromJson for DeniedInput {
        fn from_json(_value: &Json) -> std::result::Result<Self, ValidationErrors> {
            Ok(Self)
        }
    }

    impl FormRequest for DeniedInput {
        fn validate(&self) -> ValidationResult {
            panic!("validation must not run for an unauthorized request");
        }

        fn authorize(&self) -> bool {
            false
        }
    }

    struct SanitizedAuthorization {
        name: String,
    }

    impl FromJson for SanitizedAuthorization {
        fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
            Ok(Self {
                name: value
                    .get("name")
                    .and_then(Json::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            })
        }
    }

    impl FormRequest for SanitizedAuthorization {
        fn sanitize(&mut self) {
            self.name = self.name.trim().to_owned();
        }

        fn validate(&self) -> ValidationResult {
            Ok(())
        }

        fn authorize(&self) -> bool {
            self.name == "allowed"
        }
    }

    fn json_request(body: &str) -> Request {
        let mut headers = crate::Headers::new();
        headers.insert("content-type", "application/json").unwrap();
        Request::new(
            crate::Method::new("POST").unwrap(),
            "/",
            headers,
            body.as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn form_request_authorizes_before_validation() {
        let error = json_request("{}").form_request::<DeniedInput>().unwrap_err();
        assert_eq!(error.status_code(), 403);
    }

    #[test]
    fn form_request_authorization_observes_sanitized_input() {
        let input = json_request(r#"{"name":" allowed "}"#)
            .form_request::<SanitizedAuthorization>()
            .unwrap();
        assert_eq!(input.name, "allowed");
    }
}
