use crate::{AuthError, ErrorKind, Result};
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use std::fmt;
use zeroize::Zeroizing;

pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([REDACTED])")
    }
}

pub trait PasswordService: Send + Sync {
    fn hash(&self, password: &Secret) -> Result<String>;
    fn verify(&self, password: &Secret, encoded_hash: &str) -> Result<bool>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Argon2Passwords;

impl PasswordService for Argon2Passwords {
    fn hash(&self, password: &Secret) -> Result<String> {
        Argon2::default()
            .hash_password(password.expose().as_bytes())
            .map(|hash| hash.to_string())
            .map_err(crypto_error)
    }

    fn verify(&self, password: &Secret, encoded_hash: &str) -> Result<bool> {
        let hash = PasswordHash::new(encoded_hash).map_err(crypto_error)?;
        match Argon2::default().verify_password(password.expose().as_bytes(), &hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(error) => Err(crypto_error(error)),
        }
    }
}

fn crypto_error(error: impl fmt::Display) -> AuthError {
    AuthError::new(
        ErrorKind::Crypto,
        format!("password operation failed: {error}"),
    )
}
