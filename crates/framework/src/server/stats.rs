use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Low-overhead counters; snapshots are approximate while requests are active.
#[derive(Debug, Clone, Default)]
pub struct ServerStats(Arc<Counters>);
#[derive(Debug, Default)]
struct Counters {
    accepted: AtomicUsize,
    rejected: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
}
#[derive(Debug, Clone, Copy)]
pub struct ServerSnapshot {
    pub accepted: usize,
    pub rejected: usize,
    pub completed: usize,
    pub failed: usize,
}
impl ServerStats {
    pub fn snapshot(&self) -> ServerSnapshot {
        ServerSnapshot {
            accepted: self.0.accepted.load(Ordering::Relaxed),
            rejected: self.0.rejected.load(Ordering::Relaxed),
            completed: self.0.completed.load(Ordering::Relaxed),
            failed: self.0.failed.load(Ordering::Relaxed),
        }
    }
    pub(super) fn accept(&self) {
        self.0.accepted.fetch_add(1, Ordering::Relaxed);
    }
    pub(super) fn reject(&self) {
        self.0.rejected.fetch_add(1, Ordering::Relaxed);
    }
    pub(super) fn finish(&self, success: bool) {
        if success {
            self.0.completed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.0.failed.fetch_add(1, Ordering::Relaxed);
        }
    }
}
