use crate::{DatabaseError, ErrorKind};
use rusqlite::ErrorCode;

pub(super) fn map_error(error: rusqlite::Error) -> DatabaseError {
    let code = error
        .sqlite_extended_error_code()
        .map(|code| code.to_string());
    let kind = match error.sqlite_error_code() {
        Some(ErrorCode::ConstraintViolation) => ErrorKind::Constraint,
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => ErrorKind::Serialization,
        Some(ErrorCode::CannotOpen | ErrorCode::NotADatabase) => ErrorKind::Connection,
        _ => match error {
            rusqlite::Error::InvalidPath(_) => ErrorKind::Configuration,
            rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..)
            | rusqlite::Error::Utf8Error(..)
            | rusqlite::Error::InvalidColumnType(..) => ErrorKind::Decode,
            _ => ErrorKind::Query,
        },
    };
    let mapped = DatabaseError::new(kind, error.to_string());
    match code {
        Some(code) => mapped.with_code(code),
        None => mapped,
    }
}
