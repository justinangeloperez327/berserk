//! Authentication and authorization contracts with vetted cryptographic primitives.
#![forbid(unsafe_code)]

mod authorization;
mod error;
mod identity;
mod password;
mod principal;
mod session;

pub use authorization::{Ability, Decision, Gate, Policy};
pub use error::{AuthError, ErrorKind, Result};
pub use identity::{IdentityProvider, IdentityRecord, PasswordAuthenticator};
pub use password::{Argon2Passwords, PasswordService, Secret};
pub use principal::Principal;
pub use session::{
    Guard, MemorySessionStore, SessionManager, SessionRecord, SessionStore, SessionToken,
    TokenDigest,
};
