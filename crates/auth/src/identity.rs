use crate::{AuthError, ErrorKind, PasswordService, Principal, Result, Secret};

#[derive(Clone, Eq, PartialEq)]
pub struct IdentityRecord {
    pub principal: Principal,
    pub password_hash: String,
    pub enabled: bool,
}

impl std::fmt::Debug for IdentityRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IdentityRecord")
            .field("principal", &self.principal)
            .field("password_hash", &"[REDACTED]")
            .field("enabled", &self.enabled)
            .finish()
    }
}

pub trait IdentityProvider: Send + Sync {
    fn find_by_identifier(&self, identifier: &str) -> Result<Option<IdentityRecord>>;
}

/// Verifies password credentials while using a dummy hash for unknown users.
pub struct PasswordAuthenticator<P, H> {
    provider: P,
    passwords: H,
    dummy_hash: String,
}

impl<P: IdentityProvider, H: PasswordService> PasswordAuthenticator<P, H> {
    pub fn new(provider: P, passwords: H) -> Result<Self> {
        let dummy_hash = passwords.hash(&Secret::new("berserk-auth-dummy-password"))?;
        Ok(Self {
            provider,
            passwords,
            dummy_hash,
        })
    }

    pub fn authenticate(&self, identifier: &str, password: &Secret) -> Result<Principal> {
        if identifier.trim().is_empty() {
            return Err(invalid_credentials());
        }
        let identity = self.provider.find_by_identifier(identifier)?;
        let hash = identity
            .as_ref()
            .map_or(self.dummy_hash.as_str(), |record| {
                record.password_hash.as_str()
            });
        let verified = self.passwords.verify(password, hash)?;
        match identity {
            Some(record) if record.enabled && verified => Ok(record.principal),
            _ => Err(invalid_credentials()),
        }
    }
}

fn invalid_credentials() -> AuthError {
    AuthError::new(ErrorKind::InvalidCredentials, "credentials are invalid")
}
