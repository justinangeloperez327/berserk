use berserk_auth::{
    AuthError, ErrorKind, Guard, MemoryTokenStore, Principal, Result, SessionManager,
    SessionRecord, SessionStore, TokenDigest, TokenManager, TokenRecord, TokenStore,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::Arc, time::Duration};

#[test]
fn malformed_and_unknown_tokens_fail_without_exposing_input() {
    let tokens = TokenManager::new(MemoryTokenStore::default());
    let well_formed = format!("bst_{}", "0".repeat(64));
    let inputs = [
        String::new(), "secret".to_owned(), "bst_".to_owned(),
        format!("bst_{}", "a".repeat(63)), format!("bst_{}", "a".repeat(65)),
        format!("bst_{}", "A".repeat(64)), format!("bst_{}", "z".repeat(64)),
        format!("bst_{}", "é".repeat(32)), format!("bst_{}", "a".repeat(1_000_000)),
        format!("{}\n", well_formed), format!("Bearer {}", well_formed),
        well_formed,
    ];
    for input in inputs {
        assert!(tokens.authenticate(&input, 0).unwrap().is_none());
        let error = tokens.resolve(&input, 0).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidToken);
        assert_eq!(error.to_string(), "token authentication failed");
        assert!(!format!("{error:?}").contains("secret"));
    }
}

struct FailingTokenStore { panic_on_get: bool }
impl TokenStore for FailingTokenStore {
    fn put(&mut self, _: TokenDigest, _: TokenRecord) -> Result<()> { Err(store_error()) }
    fn get(&mut self, _: &TokenDigest) -> Result<Option<TokenRecord>> {
        assert!(!self.panic_on_get, "simulated store panic");
        Err(store_error())
    }
    fn revoke(&mut self, _: &TokenDigest, _: u64) -> Result<()> { Err(store_error()) }
    fn remove_expired(&mut self, _: u64) -> Result<u64> { Err(store_error()) }
}

fn store_error() -> AuthError { AuthError::new(ErrorKind::Store, "store unavailable") }

#[test]
fn token_store_failures_are_not_silently_treated_as_guests() {
    let tokens = TokenManager::new(FailingTokenStore { panic_on_get: false });
    let user = Principal::new("user:1").unwrap();
    assert_eq!(tokens.issue(user, ["read"], 0).unwrap_err().kind(), ErrorKind::Store);
    let bearer = format!("bst_{}", "0".repeat(64));
    assert_eq!(tokens.authenticate(&bearer, 0).unwrap_err().kind(), ErrorKind::Store);
    assert_eq!(tokens.revoke(&TokenDigest::from_bytes([0; 32]), 0).unwrap_err().kind(), ErrorKind::Store);
    assert_eq!(tokens.prune(0).unwrap_err().kind(), ErrorKind::Store);
}

#[test]
fn poisoned_token_store_fails_closed_without_a_second_panic() {
    let tokens = TokenManager::new(FailingTokenStore { panic_on_get: true });
    let bearer = format!("bst_{}", "0".repeat(64));
    assert!(catch_unwind(AssertUnwindSafe(|| tokens.authenticate(&bearer, 0))).is_err());
    assert_eq!(tokens.authenticate(&bearer, 0).unwrap_err().kind(), ErrorKind::Store);
}

struct PanickingSessionStore;
impl SessionStore for PanickingSessionStore {
    fn put(&mut self, _: TokenDigest, _: SessionRecord) -> Result<()> { Ok(()) }
    fn get(&mut self, _: &TokenDigest) -> Result<Option<SessionRecord>> { panic!("simulated store panic") }
    fn remove(&mut self, _: &TokenDigest) -> Result<()> { Ok(()) }
    fn remove_expired(&mut self, _: u64) -> Result<u64> { Ok(0) }
}

#[test]
fn poisoned_session_store_also_fails_closed() {
    let sessions = SessionManager::new(PanickingSessionStore, Duration::from_secs(60)).unwrap();
    let bearer = "0".repeat(64);
    assert!(catch_unwind(AssertUnwindSafe(|| sessions.authenticate(&bearer, 0))).is_err());
    assert_eq!(sessions.authenticate(&bearer, 0).unwrap_err().kind(), ErrorKind::Store);
}

#[test]
fn shared_token_manager_handles_concurrent_issuance_and_revocation() {
    let tokens = Arc::new(TokenManager::new(MemoryTokenStore::new(16).unwrap()));
    let handles: Vec<_> = (0..8).map(|index| {
        let tokens = tokens.clone();
        std::thread::spawn(move || {
            let subject = format!("user:{index}");
            let token = tokens.issue(Principal::new(&subject).unwrap(), ["read"], 0).unwrap();
            assert_eq!(tokens.authenticate(token.expose(), 0).unwrap().unwrap().subject(), subject);
            tokens.revoke(&token.digest(), 1).unwrap();
            assert!(tokens.authenticate(token.expose(), 2).unwrap().is_none());
        })
    }).collect();
    for handle in handles { handle.join().unwrap(); }
    assert_eq!(tokens.prune(2).unwrap(), 8);
}
