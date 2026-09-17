use berserk_validation::{sanitize, ValidateInput, ValidationErrors};

#[derive(Debug)]
struct Input {
    name: String,
}

impl ValidateInput for Input {
    fn sanitize(&mut self) {
        sanitize::trim(&mut self.name);
    }

    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        errors.required("name", Some(&self.name));
        errors.finish()
    }
}

#[test]
fn sanitization_can_run_before_validation() {
    let mut input = Input {
        name: "  Berserk  ".into(),
    };

    input.sanitize();
    input.validate().unwrap();

    assert_eq!(input.name, "Berserk");
}

#[test]
fn validation_errors_expose_collection_helpers() {
    let mut errors = ValidationErrors::default();
    errors.required("name", Some(""));

    assert_eq!(errors.len(), 1);
    assert!(!errors.is_empty());
    assert_eq!(errors.iter().next().unwrap().field, "name");
}
