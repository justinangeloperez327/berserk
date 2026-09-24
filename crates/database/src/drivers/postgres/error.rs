use crate::{DatabaseError, ErrorKind};

pub(super) fn map_error(error: postgres::Error) -> DatabaseError {
    let code = error.code().map(|code| code.code().to_owned());
    let kind = match code.as_deref() {
        Some("23505") => ErrorKind::UniqueViolation,
        Some("23503") => ErrorKind::ForeignKeyViolation,
        Some("23502") => ErrorKind::NotNullViolation,
        Some(code) if code.starts_with("23") => ErrorKind::Constraint,
        Some("40001" | "40P01") => ErrorKind::Serialization,
        Some(code) if code.starts_with("08") => ErrorKind::Connection,
        _ if error.is_closed() => ErrorKind::Connection,
        _ => ErrorKind::Query,
    };
    let mapped = DatabaseError::new(kind, error.to_string());
    match code {
        Some(code) => mapped.with_code(code),
        None => mapped,
    }
}
