use crate::{AuthError, ErrorKind, Principal, Result};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Ability(String);

impl Ability {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-' | b'_')
            })
        {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                "ability names must be nonempty ASCII identifiers",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    Allow,
    Deny,
}

type Rule = Arc<dyn Fn(&Principal) -> Decision + Send + Sync>;

#[derive(Default)]
pub struct Gate {
    rules: HashMap<Ability, Rule>,
}

impl Gate {
    pub fn define(
        &mut self,
        ability: Ability,
        rule: impl Fn(&Principal) -> Decision + Send + Sync + 'static,
    ) -> Result<()> {
        if self.rules.contains_key(&ability) {
            return Err(AuthError::new(
                ErrorKind::Configuration,
                format!("ability `{}` is already defined", ability.as_str()),
            ));
        }
        self.rules.insert(ability, Arc::new(rule));
        Ok(())
    }
    pub fn allows(&self, principal: &Principal, ability: &Ability) -> bool {
        matches!(
            self.rules.get(ability).map(|rule| rule(principal)),
            Some(Decision::Allow)
        )
    }
    pub fn authorize(&self, principal: &Principal, ability: &Ability) -> Result<()> {
        if self.allows(principal, ability) {
            Ok(())
        } else {
            Err(AuthError::new(
                ErrorKind::Forbidden,
                "action is not authorized",
            ))
        }
    }
}

pub trait Policy<Resource> {
    fn authorize(&self, principal: &Principal, action: &Ability, resource: &Resource) -> Decision;
}
