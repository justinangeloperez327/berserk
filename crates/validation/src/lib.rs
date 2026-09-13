//! Explicit validation contracts; no reflection or third-party dependencies.
#![forbid(unsafe_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationErrors(pub Vec<FieldError>);
impl ValidationErrors {
    pub fn add(&mut self, field: &str, code: &str, message: &str) {
        self.0.push(FieldError {
            field: field.into(),
            code: code.into(),
            message: message.into(),
        });
    }
    pub fn required(&mut self, field: &str, value: Option<&str>) {
        if value.map_or(true, |s| s.trim().is_empty()) {
            self.add(field, "required", "This field is required.");
        }
    }
    pub fn length(&mut self, field: &str, value: &str, min: usize, max: usize) {
        let count = value.chars().count();
        if count < min || count > max {
            self.add(
                field,
                "length",
                "The field length is outside the allowed range.",
            );
        }
    }
    pub fn range(&mut self, field: &str, value: i64, min: i64, max: i64) {
        if value < min || value > max {
            self.add(field, "range", "The value is outside the allowed range.");
        }
    }
    pub fn finish(self) -> Result<(), Self> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }
}
pub trait ValidateInput {
    fn validate(&self) -> Result<(), ValidationErrors>;
}
