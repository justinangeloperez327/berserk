use berserk_database::{DatabaseError, ErrorKind};

#[test]
fn integrity_categories_share_constraint_semantics() {
    for kind in [
        ErrorKind::Constraint,
        ErrorKind::UniqueViolation,
        ErrorKind::ForeignKeyViolation,
        ErrorKind::NotNullViolation,
    ] {
        let error = DatabaseError::new(kind, "private database detail");
        assert!(error.is_constraint_violation());
        assert!(!error.is_retryable());
    }
}

#[test]
fn retryable_categories_are_explicit() {
    for kind in [ErrorKind::Timeout, ErrorKind::Serialization] {
        let error = DatabaseError::new(kind, "temporary failure");
        assert!(error.is_retryable());
        assert!(!error.is_constraint_violation());
    }
}

#[test]
fn backend_codes_remain_available_without_application_string_parsing() {
    let error = DatabaseError::new(ErrorKind::UniqueViolation, "duplicate key").with_code("23505");
    assert_eq!(error.kind(), &ErrorKind::UniqueViolation);
    assert_eq!(error.code(), Some("23505"));
}
