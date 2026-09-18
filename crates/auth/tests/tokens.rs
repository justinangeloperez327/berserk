use berserk_auth::{
    ErrorKind, Guard, MemorySessionStore, MemoryTokenStore, Principal, SessionManager, TokenDigest,
    TokenManager, TokenRecord, TokenStore,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

fn user() -> Principal {
    Principal::new("user:1").unwrap().with_role("admin")
}

#[test]
fn tokens_are_unique_scoped_and_reusable_until_revoked() {
    let tokens = TokenManager::new(MemoryTokenStore::default());
    let mut issued = HashSet::new();
    for _ in 0..64 {
        let token = tokens
            .issue(user(), ["users.read", "users.update"], 100)
            .unwrap();
        assert!(issued.insert(token.expose().to_owned()));
        assert!(token.expose().starts_with("bst_"));
        assert_eq!(token.expose().len(), 68);
        for now in [100, 101, u64::MAX] {
            let principal = tokens.authenticate(token.expose(), now).unwrap().unwrap();
            assert_eq!(principal.subject(), "user:1");
            assert!(principal.has_role("admin"));
            assert!(principal.can("users.read"));
            assert!(!principal.can("users.delete"));
            assert_eq!(
                principal.abilities().collect::<Vec<_>>(),
                ["users.read", "users.update"]
            );
        }
        tokens.revoke(&token.digest(), 102).unwrap();
        assert!(tokens.authenticate(token.expose(), 103).unwrap().is_none());
        assert_eq!(
            tokens.resolve(token.expose(), 103).unwrap_err().kind(),
            ErrorKind::RevokedToken
        );
        // Moving the clock backwards must not undo revocation.
        assert!(tokens.authenticate(token.expose(), 100).unwrap().is_none());
        tokens.revoke(&token.digest(), 104).unwrap();
    }
}

#[test]
fn expiration_is_exclusive_and_overflow_is_rejected() {
    let tokens =
        TokenManager::with_ttl(MemoryTokenStore::default(), Duration::from_secs(10)).unwrap();
    let token = tokens.issue(user(), ["read"], 100).unwrap();
    assert!(tokens.authenticate(token.expose(), 99).unwrap().is_none());
    assert!(tokens.authenticate(token.expose(), 109).unwrap().is_some());
    assert_eq!(
        tokens.resolve(token.expose(), 110).unwrap_err().kind(),
        ErrorKind::ExpiredToken
    );
    assert!(tokens.authenticate(token.expose(), 110).unwrap().is_none());
    assert_eq!(
        tokens.issue(user(), ["read"], u64::MAX).unwrap_err().kind(),
        ErrorKind::Configuration
    );
    assert!(TokenManager::with_ttl(MemoryTokenStore::default(), Duration::ZERO).is_err());
    assert!(TokenManager::with_ttl(MemoryTokenStore::default(), Duration::from_nanos(1)).is_err());
}

#[test]
fn scopes_cannot_be_widened_or_interpreted_as_wildcards() {
    let tokens = TokenManager::new(MemoryTokenStore::default());
    let token = tokens.issue(user(), ["users.read"], 0).unwrap();
    let scoped = tokens.resolve(token.expose(), 0).unwrap();
    assert_eq!(
        tokens
            .issue(scoped.clone(), ["users.update"], 1)
            .unwrap_err()
            .kind(),
        ErrorKind::Forbidden
    );
    assert!(tokens.issue(scoped, ["users.read"], 1).is_ok());
    let empty = tokens.issue(user(), [] as [&str; 0], 0).unwrap();
    assert!(!tokens.resolve(empty.expose(), 0).unwrap().can("users.read"));
    for scope in ["*", "users.*", "", "users read"] {
        assert_eq!(
            tokens.issue(user(), [scope], 0).unwrap_err().kind(),
            ErrorKind::Configuration
        );
    }
}

#[test]
fn memory_capacity_pruning_and_collision_rejection_preserve_existing_tokens() {
    assert!(MemoryTokenStore::new(0).is_err());
    let tokens = TokenManager::new(MemoryTokenStore::new(1).unwrap());
    let token = tokens.issue(user(), ["read"], 0).unwrap();
    assert_eq!(
        tokens.issue(user(), ["read"], 1).unwrap_err().kind(),
        ErrorKind::Store
    );
    assert!(tokens.authenticate(token.expose(), 1).unwrap().is_some());
    tokens.revoke(&token.digest(), 2).unwrap();
    assert_eq!(tokens.prune(2).unwrap(), 1);
    assert!(tokens.issue(user(), ["read"], 2).is_ok());
    let expiring =
        TokenManager::with_ttl(MemoryTokenStore::new(1).unwrap(), Duration::from_secs(1)).unwrap();
    expiring.issue(user(), ["read"], 0).unwrap();
    assert_eq!(expiring.prune(1).unwrap(), 1);
    assert!(expiring.issue(user(), ["read"], 1).is_ok());

    let mut store = MemoryTokenStore::default();
    let digest = TokenDigest::from_bytes([1; 32]);
    let record = TokenRecord {
        principal: user(),
        issued_at: 0,
        expires_at: None,
        revoked_at: Some(1),
    };
    store.put(digest, record.clone()).unwrap();
    let mut replacement = record.clone();
    replacement.revoked_at = None;
    assert_eq!(
        store.put(digest, replacement).unwrap_err().kind(),
        ErrorKind::Store
    );
    assert_eq!(store.get(&digest).unwrap(), Some(record));
}

#[test]
fn session_credentials_and_api_credentials_are_separate() {
    let tokens = TokenManager::new(MemoryTokenStore::default());
    let sessions =
        SessionManager::new(MemorySessionStore::default(), Duration::from_secs(60)).unwrap();
    let api = tokens.issue(user(), ["read"], 0).unwrap();
    let session = sessions.issue(user(), 0).unwrap();
    assert!(tokens.authenticate(session.expose(), 0).unwrap().is_none());
    assert!(sessions.authenticate(api.expose(), 0).unwrap().is_none());
}

type Captured = Arc<Mutex<Option<(TokenDigest, TokenRecord)>>>;
struct Capture(Captured);
impl TokenStore for Capture {
    fn put(&mut self, digest: TokenDigest, token: TokenRecord) -> berserk_auth::Result<()> {
        *self.0.lock().unwrap() = Some((digest, token));
        Ok(())
    }
    fn get(&mut self, _: &TokenDigest) -> berserk_auth::Result<Option<TokenRecord>> {
        Ok(None)
    }
    fn revoke(&mut self, _: &TokenDigest, _: u64) -> berserk_auth::Result<()> {
        Ok(())
    }
    fn remove_expired(&mut self, _: u64) -> berserk_auth::Result<u64> {
        Ok(0)
    }
}

#[test]
fn storage_receives_only_digest_and_redacted_metadata() {
    let captured = Arc::new(Mutex::new(None));
    let tokens =
        TokenManager::with_ttl(Capture(captured.clone()), Duration::from_secs(60)).unwrap();
    let token = tokens
        .issue(user().with_claim("secret", "claim-secret"), ["read"], 100)
        .unwrap();
    let captured = captured.lock().unwrap();
    let (digest, record) = captured.as_ref().unwrap();
    assert_eq!(digest, &token.digest());
    assert_eq!(record.issued_at, 100);
    assert_eq!(record.expires_at, Some(160));
    assert_eq!(record.revoked_at, None);
    let debug = format!("{token:?} {digest:?} {record:?}");
    assert!(!debug.contains(token.expose()));
    assert!(!debug.contains("claim-secret"));
    assert!(debug.contains("[REDACTED]"));
}
