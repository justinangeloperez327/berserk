use crate::{Ability, AuthError, ErrorKind, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Clone, Eq, PartialEq)]
pub struct Principal {
    subject: String,
    roles: BTreeSet<String>,
    claims: BTreeMap<String, String>,
    abilities: Option<BTreeSet<String>>,
}

impl Principal {
    pub fn new(subject: impl Into<String>) -> Option<Self> {
        let subject = subject.into();
        if subject.trim().is_empty() || subject.len() > 1024 || subject.chars().any(char::is_control) {
            return None;
        }
        Some(Self {
            subject,
            roles: BTreeSet::new(),
            claims: BTreeMap::new(),
            abilities: None,
        })
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn roles(&self) -> impl Iterator<Item = &str> {
        self.roles.iter().map(String::as_str)
    }
    pub fn has_role(&self, role: &str) -> bool {
        valid_role(role) && self.roles.contains(role)
    }
    pub fn claim(&self, name: &str) -> Option<&str> {
        self.claims.get(name).map(String::as_str)
    }
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        let role = role.into();
        // Preserve the builder API, but never grant malformed roles.
        if valid_role(&role) {
            self.roles.insert(role);
        }
        self
    }
    pub fn with_claim(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.claims.insert(name.into(), value.into());
        self
    }

    /// Restrict this identity to exact ability names. Restrictions cannot be widened.
    /// An empty list grants no abilities; wildcard names are not supported.
    pub fn with_abilities<I, A>(mut self, abilities: I) -> Result<Self>
    where
        I: IntoIterator<Item = A>,
        A: AsRef<str>,
    {
        let mut granted = BTreeSet::new();
        for (index, name) in abilities.into_iter().enumerate() {
            if index >= 128 {
                return Err(AuthError::new(
                    ErrorKind::Configuration,
                    "too many abilities",
                ));
            }
            let ability = Ability::new(name.as_ref())?;
            if !self.permits(ability.as_str()) {
                return Err(AuthError::new(
                    ErrorKind::Forbidden,
                    "cannot widen ability restrictions",
                ));
            }
            granted.insert(ability.as_str().to_owned());
        }
        self.abilities = Some(granted);
        Ok(self)
    }

    /// Check an explicitly granted ability. Roles and claims never imply abilities.
    pub fn can(&self, ability: &str) -> bool {
        self.abilities
            .as_ref()
            .is_some_and(|abilities| abilities.contains(ability))
    }

    pub fn abilities(&self) -> impl Iterator<Item = &str> {
        self.abilities
            .iter()
            .flat_map(|abilities| abilities.iter().map(String::as_str))
    }

    /// Whether an ability is within this identity's scope ceiling.
    /// Unscoped identities still require a gate or policy decision.
    pub fn permits(&self, ability: &str) -> bool {
        self.abilities
            .as_ref()
            .is_none_or(|abilities| abilities.contains(ability))
    }
}

fn valid_role(role: &str) -> bool {
    !role.is_empty()
        && role.len() <= 128
        && role.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-' | b'_')
        })
}

impl fmt::Debug for Principal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Principal")
            .field("subject", &"[REDACTED]")
            .field("roles", &self.roles)
            .field("claims", &"[REDACTED]")
            .field("abilities", &self.abilities)
            .finish()
    }
}
