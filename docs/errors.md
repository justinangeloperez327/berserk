# Error contracts

Berserk separates internal diagnostics from stable application-facing error categories.

## Database categories

`DatabaseError::kind()` is the portable classification layer. Applications should branch on `ErrorKind`, not parse driver messages or vendor codes.

Integrity failures use these categories:

- `UniqueViolation` — UNIQUE or primary-key conflict.
- `ForeignKeyViolation` — referenced or referencing row prevents the operation.
- `NotNullViolation` — a required column received NULL.
- `Constraint` — another integrity constraint, or a backend constraint that cannot be classified more precisely.

`DatabaseError::is_constraint_violation()` accepts all four categories. Vendor error codes remain available through `code()` for diagnostics, but are not the portable application contract.

`Timeout` and `Serialization` are marked retryable by `is_retryable()`. Retry policy still belongs to the caller; the helper only classifies the failure.

## HTTP mapping

The framework maps database not-found to 404, invalid input to 400, integrity failures to 409, and timeout to 503. Other database failures remain internal server errors.

Public 5xx responses are redacted. Database messages, SQL details, configuration values, credentials, and backend diagnostics must not be copied into client responses. Internal errors retain their original source for logging and diagnosis.

## Compatibility

The existing `Constraint` variant remains valid throughout Berserk 1.x. The more specific integrity categories are additive and allow drivers to provide portable semantics without requiring applications to parse PostgreSQL SQLSTATE, MySQL numeric codes, or SQLite extended result codes.
