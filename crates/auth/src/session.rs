use crate::{AuthError, ErrorKind, Principal, Result};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt, sync::Mutex, time::Duration};
use zeroize::{Zeroize, Zeroizing};

pub struct SessionToken(Zeroizing<String>);

impl SessionToken {
    pub fn generate() -> Self {
        let mut bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token = hex(&bytes);
        bytes.zeroize();
        Self(Zeroizing::new(token))
    }
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AuthError::new(
                ErrorKind::InvalidCredentials,
                "invalid bearer token format",
            ));
        }
        Ok(Self(Zeroizing::new(value)))
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

#[derive(Default)]
pub struct MemorySessionStore {
    sessions: HashMap<TokenDigest, SessionRecord>,
}

impl SessionStore for MemorySessionStore {
    fn put(&mut self, digest: TokenDigest, session: SessionRecord) -> Result<()> {
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

pub struct SessionManager<S> {
    store: Mutex<S>,
    ttl: Duration,
}

impl<S: SessionStore> SessionManager<S> {
    pub fn new(store: S, ttl: Duration) -> Result<Self> {
        if ttl.is_zero() {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "session TTL must be greater than zero",
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
        let token = SessionToken::generate();
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
        let Some(session) = self.lock()?.get(&digest)? else {
            return Ok(None);
        };
        if session.expires_at <= now {
            self.lock()?.remove(&digest)?;
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
