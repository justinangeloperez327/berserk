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
        if value.is_none_or(|value| value.trim().is_empty()) {
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

    pub fn min_length(&mut self, field: &str, value: &str, min: usize) {
        if value.chars().count() < min {
            self.add(field, "min_length", "The field is shorter than allowed.");
        }
    }

    pub fn max_length(&mut self, field: &str, value: &str, max: usize) {
        if value.chars().count() > max {
            self.add(field, "max_length", "The field is longer than allowed.");
        }
    }

    pub fn range(&mut self, field: &str, value: i64, min: i64, max: i64) {
        if value < min || value > max {
            self.add(field, "range", "The value is outside the allowed range.");
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FieldError> {
        self.0.iter()
    }

    pub fn finish(self) -> Result<(), Self> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

/// Input normalization helpers.
///
/// These helpers normalize user input before validation. They deliberately do
/// not perform output escaping or SQL escaping; those belong to the rendering
/// and database boundaries.
pub mod sanitize {
    pub fn trim(value: &mut String) {
        let trimmed = value.trim();
        if trimmed.len() != value.len() {
            *value = trimmed.to_owned();
        }
    }

    pub fn lowercase(value: &mut String) {
        let lowered = value.to_lowercase();
        if lowered != *value {
            *value = lowered;
        }
    }

    pub fn trim_option(value: &mut Option<String>) {
        let Some(text) = value else {
            return;
        };
        trim(text);
        if text.is_empty() {
            *value = None;
        }
    }
}

pub trait ValidateInput {
    /// Normalize accepted input before semantic validation.
    ///
    /// Existing implementations do not need to define this method.
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), ValidationErrors>;
}
