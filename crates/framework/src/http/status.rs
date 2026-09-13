use super::HttpError;

/// Final response code. Informational responses and upgrades are deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const OK: Self = Self(200);
    pub const CREATED: Self = Self(201);
    pub const NO_CONTENT: Self = Self(204);
    pub const BAD_REQUEST: Self = Self(400);
    pub const NOT_FOUND: Self = Self(404);
    pub const INTERNAL_SERVER_ERROR: Self = Self(500);

    pub fn new(code: u16) -> Result<Self, HttpError> {
        if !(200..=599).contains(&code) {
            return Err(HttpError::InvalidStatus(code));
        }
        Ok(Self(code))
    }
    pub fn as_u16(self) -> u16 {
        self.0
    }
    pub fn allows_body(self) -> bool {
        !matches!(self.0, 204 | 205 | 304)
    }
}
impl TryFrom<u16> for StatusCode {
    type Error = HttpError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
