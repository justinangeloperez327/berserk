use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Principal {
    subject: String,
    roles: BTreeSet<String>,
    claims: BTreeMap<String, String>,
}

impl Principal {
    pub fn new(subject: impl Into<String>) -> Option<Self> {
        let subject = subject.into();
        if subject.trim().is_empty() {
            return None;
        }
        Some(Self {
            subject,
            roles: BTreeSet::new(),
            claims: BTreeMap::new(),
        })
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn roles(&self) -> impl Iterator<Item = &str> {
        self.roles.iter().map(String::as_str)
    }
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }
    pub fn claim(&self, name: &str) -> Option<&str> {
        self.claims.get(name).map(String::as_str)
    }
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.insert(role.into());
        self
    }
    pub fn with_claim(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.claims.insert(name.into(), value.into());
        self
    }
}
