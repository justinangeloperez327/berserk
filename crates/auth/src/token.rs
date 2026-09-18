use crate::{AuthError, ErrorKind, Guard, Principal, Result, SessionToken, TokenDigest};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt, sync::Mutex, time::Duration};
use zeroize::Zeroizing;

/// An opaque API credential. Only issuance returns a recoverable plaintext value.
/// The prefix separates API credentials from existing session credentials.
pub struct ApiToken(Zeroizing<String>);

impl ApiToken {
    fn generate() -> Result<Self> {
        let secret = SessionToken::try_generate()?;
        Ok(Self(Zeroizing::new(format!("bst_{}", secret.expose()))))
    }

    fn parse(value: &str) -> Result<Self> {
        if value.len() != 68
            || !value.starts_with("bst_")
            || !value.as_bytes()[4..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(token_error(ErrorKind::InvalidToken));
        }
        Ok(Self(Zeroizing::new(value.to_owned())))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn digest(&self) -> TokenDigest {
        TokenDigest::from_bytes(Sha256::digest(self.0.as_bytes()).into())
    }
}

impl fmt::Debug for ApiToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApiToken([REDACTED])")
    }
}

/// Stored metadata contains a scoped principal, never a plaintext credential.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenRecord {
    pub principal: Principal,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub revoked_at: Option<u64>,
}

/// Trusted persistence boundary. Inserts must reject collisions, and revocation
/// must atomically and permanently mark an existing record as revoked.
/// Lookups use a SHA-256 digest of the entire 256-bit random credential; no raw
/// secret comparison is needed. Implementations must not log credentials.
pub trait TokenStore: Send {
    fn put(&mut self, digest: TokenDigest, token: TokenRecord) -> Result<()>;
    fn get(&mut self, digest: &TokenDigest) -> Result<Option<TokenRecord>>;
    fn revoke(&mut self, digest: &TokenDigest, now: u64) -> Result<()>;
    fn remove_expired(&mut self, now: u64) -> Result<u64>;
}

/// Bounded development storage. Explicit pruning also removes revoked records.
pub struct MemoryTokenStore {
    tokens: HashMap<TokenDigest, TokenRecord>,
    max_records: usize,
}

impl MemoryTokenStore {
    pub fn new(max_records: usize) -> Result<Self> {
        if max_records == 0 {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "token capacity must be positive",
            ));
        }
        Ok(Self {
            tokens: HashMap::new(),
            max_records,
        })
    }
}

impl Default for MemoryTokenStore {
    fn default() -> Self {
        Self {
            tokens: HashMap::new(),
            max_records: 10_000,
        }
    }
}

impl TokenStore for MemoryTokenStore {
    fn put(&mut self, digest: TokenDigest, token: TokenRecord) -> Result<()> {
        if self.tokens.len() >= self.max_records || self.tokens.contains_key(&digest) {
            return Err(AuthError::new(
                ErrorKind::Store,
                "token store capacity or key conflict",
            ));
        }
        self.tokens.insert(digest, token);
        Ok(())
    }

    fn get(&mut self, digest: &TokenDigest) -> Result<Option<TokenRecord>> {
        Ok(self.tokens.get(digest).cloned())
    }

    fn revoke(&mut self, digest: &TokenDigest, now: u64) -> Result<()> {
        if let Some(token) = self.tokens.get_mut(digest) {
            token.revoked_at.get_or_insert(now);
        }
        Ok(())
    }

    fn remove_expired(&mut self, now: u64) -> Result<u64> {
        let before = self.tokens.len();
        self.tokens.retain(|_, token| {
            token.revoked_at.is_none() && token.expires_at.is_none_or(|expiry| expiry > now)
        });
        Ok((before - self.tokens.len()) as u64)
    }
}

pub struct TokenManager<S> {
    store: Mutex<S>,
    ttl: Option<Duration>,
}

impl<S: TokenStore> TokenManager<S> {
    /// Issue tokens without expiration. Use `with_ttl` for expiring tokens.
    pub fn new(store: S) -> Self {
        Self {
            store: Mutex::new(store),
            ttl: None,
        }
    }

    pub fn with_ttl(store: S, ttl: Duration) -> Result<Self> {
        if ttl.as_secs() == 0 {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "token TTL must be at least one second",
            ));
        }
        Ok(Self {
            store: Mutex::new(store),
            ttl: Some(ttl),
        })
    }

    /// The trusted issuer chooses scopes; a scoped principal cannot mint broader scopes.
    /// Tokens are reusable bearer credentials until expiration or revocation.
    pub fn issue<I, A>(&self, principal: Principal, abilities: I, now: u64) -> Result<ApiToken>
    where
        I: IntoIterator<Item = A>,
        A: AsRef<str>,
    {
        let principal = principal.with_abilities(abilities)?;
        let expires_at = self
            .ttl
            .map(|ttl| {
                now.checked_add(ttl.as_secs()).ok_or_else(|| {
                    AuthError::new(ErrorKind::Configuration, "token expiry overflow")
                })
            })
            .transpose()?;
        let token = ApiToken::generate()?;
        self.lock()?.put(
            token.digest(),
            TokenRecord {
                principal,
                issued_at: now,
                expires_at,
                revoked_at: None,
            },
        )?;
        Ok(token)
    }

    /// Detailed internal diagnostics. HTTP code should authenticate through `Guard`,
    /// which collapses invalid, expired, and revoked credentials to `None`.
    pub fn resolve(&self, bearer_token: &str, now: u64) -> Result<Principal> {
        let token = ApiToken::parse(bearer_token)?;
        let record = self
            .lock()?
            .get(&token.digest())?
            .ok_or_else(|| token_error(ErrorKind::InvalidToken))?;
        if record.revoked_at.is_some() {
            return Err(token_error(ErrorKind::RevokedToken));
        }
        if now < record.issued_at {
            return Err(token_error(ErrorKind::InvalidToken));
        }
        if record.expires_at.is_some_and(|expiry| now >= expiry) {
            return Err(token_error(ErrorKind::ExpiredToken));
        }
        Ok(record.principal)
    }

    /// Revoke by digest so an application need not retain or recover plaintext.
    pub fn revoke(&self, digest: &TokenDigest, now: u64) -> Result<()> {
        self.lock()?.revoke(digest, now)
    }

    pub fn prune(&self, now: u64) -> Result<u64> {
        self.lock()?.remove_expired(now)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, S>> {
        self.store
            .lock()
            .map_err(|_| AuthError::new(ErrorKind::Store, "token store lock poisoned"))
    }
}

impl<S: TokenStore + 'static> Guard for TokenManager<S> {
    fn authenticate(&self, bearer_token: &str, now: u64) -> Result<Option<Principal>> {
        match self.resolve(bearer_token, now) {
            Ok(principal) => Ok(Some(principal)),
            Err(error) if error.is_authentication_failure() => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn token_error(kind: ErrorKind) -> AuthError {
    AuthError::new(kind, "token authentication failed")
}
