use crate::{AuthError, ErrorKind, Principal, Result};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
    time::Duration,
};
use zeroize::Zeroizing;

pub struct SessionToken(Zeroizing<String>);

impl SessionToken {
    pub fn generate() -> Self {
        Self::try_generate().expect("operating system randomness unavailable")
    }
    /// Fallible generation for servers, without panicking on entropy failure.
    pub fn try_generate() -> Result<Self> {
        let mut bytes = Zeroizing::new([0_u8; 32]);
        OsRng.try_fill_bytes(bytes.as_mut()).map_err(|_| {
            AuthError::new(ErrorKind::Crypto, "operating system randomness unavailable")
        })?;
        Ok(Self(Zeroizing::new(hex(bytes.as_ref()))))
    }
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AuthError::new(
                ErrorKind::InvalidCredentials,
                "invalid bearer token format",
            ));
        }
        Ok(Self(Zeroizing::new(value.to_owned())))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn digest(&self) -> TokenDigest {
        let digest: [u8; 32] = Sha256::digest(self.0.as_bytes()).into();
        TokenDigest(digest)
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionToken([REDACTED])")
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct TokenDigest([u8; 32]);

impl TokenDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for TokenDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TokenDigest([REDACTED])")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub principal: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
}

pub trait SessionStore: Send {
    fn put(&mut self, digest: TokenDigest, session: SessionRecord) -> Result<()>;
    fn get(&mut self, digest: &TokenDigest) -> Result<Option<SessionRecord>>;
    fn remove(&mut self, digest: &TokenDigest) -> Result<()>;
    fn remove_expired(&mut self, now: u64) -> Result<u64>;
}

pub struct MemorySessionStore {
    sessions: HashMap<TokenDigest, SessionRecord>,
    max_records: usize,
}

impl MemorySessionStore {
    /// Bounded development storage. Call `SessionManager::prune` to reclaim expired entries.
    pub fn new(max_records: usize) -> Result<Self> {
        if max_records == 0 {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "session capacity must be positive",
            ));
        }
        Ok(Self {
            sessions: HashMap::new(),
            max_records,
        })
    }
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            max_records: 10_000,
        }
    }
}

impl SessionStore for MemorySessionStore {
    fn put(&mut self, digest: TokenDigest, session: SessionRecord) -> Result<()> {
        if self.sessions.len() >= self.max_records || self.sessions.contains_key(&digest) {
            return Err(AuthError::new(
                ErrorKind::Store,
                "session store capacity or key conflict",
            ));
        }
        self.sessions.insert(digest, session);
        Ok(())
    }
    fn get(&mut self, digest: &TokenDigest) -> Result<Option<SessionRecord>> {
        Ok(self.sessions.get(digest).cloned())
    }
    fn remove(&mut self, digest: &TokenDigest) -> Result<()> {
        self.sessions.remove(digest);
        Ok(())
    }
    fn remove_expired(&mut self, now: u64) -> Result<u64> {
        let before = self.sessions.len();
        self.sessions.retain(|_, session| session.expires_at > now);
        Ok((before - self.sessions.len()) as u64)
    }
}

pub trait Guard: Send + Sync + 'static {
    fn authenticate(&self, bearer_token: &str, now: u64) -> Result<Option<Principal>>;
}

impl<G: Guard + ?Sized> Guard for Arc<G> {
    fn authenticate(&self, bearer_token: &str, now: u64) -> Result<Option<Principal>> {
        (**self).authenticate(bearer_token, now)
    }
}

pub struct SessionManager<S> {
    store: Mutex<S>,
    ttl: Duration,
}

impl<S: SessionStore> SessionManager<S> {
    pub fn new(store: S, ttl: Duration) -> Result<Self> {
        if ttl.as_secs() == 0 {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "session TTL must be at least one second",
            ));
        }
        Ok(Self {
            store: Mutex::new(store),
            ttl,
        })
    }
    pub fn issue(&self, principal: Principal, now: u64) -> Result<SessionToken> {
        let expires_at = now
            .checked_add(self.ttl.as_secs())
            .ok_or_else(|| AuthError::new(ErrorKind::Configuration, "session expiry overflow"))?;
        let token = SessionToken::try_generate()?;
        self.lock()?.put(
            token.digest(),
            SessionRecord {
                principal,
                issued_at: now,
                expires_at,
            },
        )?;
        Ok(token)
    }
    pub fn revoke(&self, token: &SessionToken) -> Result<()> {
        self.lock()?.remove(&token.digest())
    }
    pub fn prune(&self, now: u64) -> Result<u64> {
        self.lock()?.remove_expired(now)
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, S>> {
        self.store
            .lock()
            .map_err(|_| AuthError::new(ErrorKind::Store, "session store lock poisoned"))
    }
}

impl<S: SessionStore + 'static> Guard for SessionManager<S> {
    fn authenticate(&self, bearer_token: &str, now: u64) -> Result<Option<Principal>> {
        let token = match SessionToken::parse(bearer_token) {
            Ok(token) => token,
            Err(_) => return Ok(None),
        };
        let digest = token.digest();
        let mut store = self.lock()?;
        let Some(session) = store.get(&digest)? else {
            return Ok(None);
        };
        if session.expires_at <= now {
            store.remove(&digest)?;
            return Ok(None);
        }
        if now < session.issued_at {
            return Ok(None);
        }
        Ok(Some(session.principal))
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}
