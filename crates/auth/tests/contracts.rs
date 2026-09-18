use berserk_auth::{
    Ability, Argon2Passwords, Decision, ErrorKind, Gate, Guard, IdentityProvider, IdentityRecord,
    MemorySessionStore, PasswordAuthenticator, PasswordService, Principal, Result, Secret,
    SessionManager,
};
use std::time::Duration;

#[test]
fn passwords_use_phc_encoded_argon2_and_verify_without_exposing_secrets() {
    let passwords = Argon2Passwords;
    let secret = Secret::new("correct horse battery staple");
    let encoded = passwords.hash(&secret).unwrap();
    let second = passwords.hash(&secret).unwrap();
    assert!(encoded.starts_with("$argon2"));
    assert_ne!(encoded, second);
    assert!(passwords.verify(&secret, &encoded).unwrap());
    assert!(!passwords.verify(&Secret::new("wrong"), &encoded).unwrap());
    assert_eq!(format!("{secret:?}"), "Secret([REDACTED])");
}

struct Users {
    identity: IdentityRecord,
}
impl IdentityProvider for Users {
    fn find_by_identifier(&self, identifier: &str) -> Result<Option<IdentityRecord>> {
        Ok((identifier == "ada@example.test").then(|| self.identity.clone()))
    }
}

#[test]
fn password_authentication_returns_one_generic_failure() {
    let passwords = Argon2Passwords;
    let identity = IdentityRecord {
        principal: Principal::new("user:9").unwrap(),
        password_hash: passwords.hash(&Secret::new("valid-password")).unwrap(),
        enabled: true,
    };
    let auth = PasswordAuthenticator::new(Users { identity }, passwords).unwrap();
    assert_eq!(
        auth.authenticate("ada@example.test", &Secret::new("valid-password"))
            .unwrap()
            .subject(),
        "user:9"
    );
    assert_eq!(
        auth.authenticate("missing@example.test", &Secret::new("wrong"))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidCredentials
    );
    assert_eq!(
        auth.authenticate("ada@example.test", &Secret::new("wrong"))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidCredentials
    );
}

#[test]
fn identity_debug_output_redacts_password_hashes() {
    let record = IdentityRecord {
        principal: Principal::new("user:1").unwrap(),
        password_hash: "sensitive-hash".into(),
        enabled: true,
    };
    let debug = format!("{record:?}");
    assert!(!debug.contains("sensitive-hash"));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn sessions_expire_revoke_and_never_store_raw_tokens() {
    let sessions =
        SessionManager::new(MemorySessionStore::default(), Duration::from_secs(60)).unwrap();
    let principal = Principal::new("user:42").unwrap().with_role("admin");
    let token = sessions.issue(principal, 1_000).unwrap();
    assert_eq!(token.expose().len(), 64);
    assert_eq!(format!("{token:?}"), "SessionToken([REDACTED])");
    assert_eq!(
        sessions
            .authenticate(token.expose(), 1_059)
            .unwrap()
            .unwrap()
            .subject(),
        "user:42"
    );
    assert!(sessions
        .authenticate(token.expose(), 1_060)
        .unwrap()
        .is_none());

    let token = sessions
        .issue(Principal::new("user:42").unwrap(), 2_000)
        .unwrap();
    sessions.revoke(&token).unwrap();
    assert!(sessions
        .authenticate(token.expose(), 2_001)
        .unwrap()
        .is_none());
}

#[test]
fn gates_deny_unknown_abilities_and_support_roles() {
    let edit = Ability::new("posts.update").unwrap();
    let mut gate = Gate::default();
    gate.define(edit.clone(), |principal| {
        if principal.has_role("editor") {
            Decision::Allow
        } else {
            Decision::Deny
        }
    })
    .unwrap();

    let editor = Principal::new("user:1").unwrap().with_role("editor");
    let reader = Principal::new("user:2").unwrap();
    assert!(gate.authorize(&editor, &edit).is_ok());
    assert_eq!(
        gate.authorize(&reader, &edit).unwrap_err().kind(),
        ErrorKind::Forbidden
    );
    assert!(!gate.allows(&editor, &Ability::new("posts.delete").unwrap()));
}

#[test]
fn duplicate_gate_definitions_are_rejected() {
    let ability = Ability::new("users.view").unwrap();
    let mut gate = Gate::default();
    gate.define(ability.clone(), |_| Decision::Allow).unwrap();
    assert_eq!(
        gate.define(ability, |_| Decision::Deny).unwrap_err().kind(),
        ErrorKind::Configuration
    );
}

#[test]
fn token_scopes_limit_role_based_gates() {
    let update = Ability::new("users.update").unwrap();
    let mut gate = Gate::default();
    gate.define(update.clone(), |principal| {
        if principal.has_role("admin") { Decision::Allow } else { Decision::Deny }
    }).unwrap();
    let admin = Principal::new("user:1").unwrap().with_role("admin");
    assert!(gate.allows(&admin, &update));
    assert!(!admin.can("users.update"));
    let scoped = admin.with_abilities(["users.read"]).unwrap();
    assert!(scoped.can("users.read"));
    assert!(!scoped.can("users.update"));
    assert!(!gate.allows(&scoped, &update));
    assert_eq!(scoped.clone().with_abilities(["users.update"]).unwrap_err().kind(), ErrorKind::Forbidden);
    let empty = scoped.with_abilities([] as [&str; 0]).unwrap();
    assert!(!empty.can("users.read"));
    assert!(!empty.permits("users.read"));
}

#[test]
fn disabled_and_blank_identities_return_generic_credential_failures() {
    let auth = PasswordAuthenticator::new(Users {
        identity: IdentityRecord {
            principal: Principal::new("user:1").unwrap(),
            // A disabled identity must use the dummy hash, even when this is invalid.
            password_hash: "invalid-hash".to_owned(),
            enabled: false,
        },
    }, Argon2Passwords).unwrap();
    for identifier in ["", "   ", "ada@example.test", "unknown@example.test"] {
        let error = auth.authenticate(identifier, &Secret::new("secret")).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidCredentials);
        assert_eq!(error.to_string(), "credentials are invalid");
    }
}

#[test]
fn principal_roles_abilities_and_claim_debug_fail_safely() {
    let principal = Principal::new("private-identity").unwrap()
        .with_role("admin").with_role("").with_role(" admin").with_role("admin\n")
        .with_claim("access_token", "private-secret");
    assert!(principal.has_role("admin"));
    assert_eq!(principal.roles().collect::<Vec<_>>(), vec!["admin"]);
    assert!(!principal.has_role(" admin"));
    assert_eq!(principal.claim("access_token"), Some("private-secret"));
    let debug = format!("{principal:?}");
    assert!(!debug.contains("private-secret"));
    assert!(!debug.contains("private-identity"));
    for invalid in ["", "*", "users.*", "read write", "read\n"] {
        assert!(principal.clone().with_abilities([invalid]).is_err());
    }
    assert!(principal.clone().with_abilities(["a".repeat(129)]).is_err());
    assert!(principal.with_abilities(["read"; 129]).is_err());
    assert!(Principal::new("user\nadmin").is_none());
}

#[test]
fn session_time_bounds_capacity_and_shared_guards_are_preserved() {
    assert!(SessionManager::new(MemorySessionStore::default(), Duration::ZERO).is_err());
    assert!(SessionManager::new(MemorySessionStore::default(), Duration::from_nanos(1)).is_err());
    assert!(MemorySessionStore::new(0).is_err());
    let sessions = std::sync::Arc::new(SessionManager::new(
        MemorySessionStore::new(1).unwrap(), Duration::from_secs(10)
    ).unwrap());
    let user = Principal::new("user:1").unwrap();
    assert_eq!(sessions.issue(user.clone(), u64::MAX).unwrap_err().kind(), ErrorKind::Configuration);
    let token = sessions.issue(user.clone(), 10).unwrap();
    let shared: std::sync::Arc<dyn Guard> = sessions.clone();
    assert!(shared.authenticate(token.expose(), 9).unwrap().is_none());
    assert!(shared.authenticate(token.expose(), 10).unwrap().is_some());
    assert_eq!(sessions.issue(user.clone(), 11).unwrap_err().kind(), ErrorKind::Store);
    assert_eq!(sessions.prune(20).unwrap(), 1);
    assert!(sessions.issue(user, 20).is_ok());
}
