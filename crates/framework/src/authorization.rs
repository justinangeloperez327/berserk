use crate::{Error, Request, Result};
use berserk_auth::{Ability, Decision, Gate, Policy, Principal};
use std::cell::RefCell;
thread_local! { static PRINCIPAL: RefCell<Option<Principal>> = const { RefCell::new(None) }; }

pub struct Auth;
impl Auth {
    pub fn user() -> Option<Principal> {
        PRINCIPAL.with(|p| p.borrow().clone())
    }
    pub fn check() -> bool {
        Self::user().is_some()
    }
    /// For synchronous controller execution. In async code use `Request::authorize`.
    pub fn authorize<R>(
        policy: &impl Policy<R>,
        ability: impl AsRef<str>,
        resource: &R,
    ) -> Result<()> {
        authorize(Self::user().as_ref(), policy, ability, resource)
    }
    pub fn gate(gate: &Gate, ability: &Ability) -> Result<()> {
        gate.authorize(&Self::user().ok_or_else(Error::unauthorized)?, ability)?;
        Ok(())
    }
}
fn authorize<R>(
    principal: Option<&Principal>,
    policy: &impl Policy<R>,
    ability: impl AsRef<str>,
    resource: &R,
) -> Result<()> {
    let principal = principal.ok_or_else(Error::unauthorized)?;
    let ability = Ability::new(ability.as_ref())?;
    if !principal.permits(ability.as_str()) {
        return Err(Error::forbidden());
    }
    match policy.authorize(principal, &ability, resource) {
        Decision::Allow => Ok(()),
        Decision::Deny => Err(Error::forbidden()),
    }
}
impl Request {
    pub fn authorize<R>(
        &self,
        policy: &impl Policy<R>,
        ability: impl AsRef<str>,
        resource: &R,
    ) -> Result<()> {
        authorize(self.principal(), policy, ability, resource)
    }
}
pub(crate) fn scope_principal<T>(principal: Option<Principal>, operation: impl FnOnce() -> T) -> T {
    struct Restore(Option<Principal>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PRINCIPAL.with(|p| *p.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(PRINCIPAL.with(|p| p.replace(principal)));
    operation()
}
