//! Typed, synchronous application events with explicit listener ordering.
#![forbid(unsafe_code)]

mod error;
pub use error::{DispatchError, ErrorKind, Result};

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, RwLock},
};

pub trait Event: Any + Send + Sync + 'static {
    const NAME: &'static str;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ListenerId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchReport {
    pub listeners_run: usize,
}

type Listener = Arc<dyn Fn(&dyn Any) -> Result<()> + Send + Sync>;

#[derive(Default)]
struct State {
    next_id: u64,
    listeners: HashMap<TypeId, Vec<(ListenerId, Listener)>>,
}

#[derive(Default)]
pub struct EventBus {
    state: RwLock<State>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn listen<E, F>(&self, listener: F) -> Result<ListenerId>
    where
        E: Event,
        F: Fn(&E) -> Result<()> + Send + Sync + 'static,
    {
        let wrapped: Listener = Arc::new(move |event| {
            let event = event.downcast_ref::<E>().ok_or_else(|| {
                DispatchError::new(
                    ErrorKind::TypeMismatch,
                    E::NAME,
                    None,
                    "listener received the wrong event type",
                )
            })?;
            listener(event)
        });
        let mut state = self.state.write().map_err(|_| {
            DispatchError::new(
                ErrorKind::Unavailable,
                E::NAME,
                None,
                "event listener lock poisoned",
            )
        })?;
        state.next_id = state.next_id.checked_add(1).ok_or_else(|| {
            DispatchError::new(
                ErrorKind::Capacity,
                E::NAME,
                None,
                "listener identifier capacity exhausted",
            )
        })?;
        let id = ListenerId(state.next_id);
        state
            .listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push((id, wrapped));
        Ok(id)
    }

    pub fn forget<E: Event>(&self, id: ListenerId) -> Result<bool> {
        let mut state = self.state.write().map_err(|_| {
            DispatchError::new(
                ErrorKind::Unavailable,
                E::NAME,
                Some(id),
                "event listener lock poisoned",
            )
        })?;
        let Some(listeners) = state.listeners.get_mut(&TypeId::of::<E>()) else {
            return Ok(false);
        };
        let before = listeners.len();
        listeners.retain(|(candidate, _)| *candidate != id);
        Ok(before != listeners.len())
    }

    pub fn dispatch<E: Event>(&self, event: &E) -> Result<DispatchReport> {
        let listeners = self
            .state
            .read()
            .map_err(|_| {
                DispatchError::new(
                    ErrorKind::Unavailable,
                    E::NAME,
                    None,
                    "event listener lock poisoned",
                )
            })?
            .listeners
            .get(&TypeId::of::<E>())
            .cloned()
            .unwrap_or_default();
        let mut listeners_run = 0;
        for (id, listener) in listeners {
            match catch_unwind(AssertUnwindSafe(|| listener(event))) {
                Ok(Ok(())) => listeners_run += 1,
                Ok(Err(error)) => return Err(error.with_context(E::NAME, id)),
                Err(_) => {
                    return Err(DispatchError::new(
                        ErrorKind::ListenerPanic,
                        E::NAME,
                        Some(id),
                        "event listener panicked",
                    ))
                }
            }
        }
        Ok(DispatchReport { listeners_run })
    }
}
