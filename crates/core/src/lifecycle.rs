use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Cooperative, one-way shutdown signal; it does not stop threads itself.
#[derive(Debug, Clone, Default)]
pub struct ShutdownHandle {
    requested: Arc<AtomicBool>,
}

impl ShutdownHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Idempotently request shutdown for all clones.
    pub fn shutdown(&self) {
        self.requested.store(true, Ordering::Release);
    }

    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}
