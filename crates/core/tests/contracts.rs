use berserk_core::{ShutdownHandle, State};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn state_shares_non_clone_values_across_threads() {
    let value = State::new(AtomicUsize::new(0));
    let cloned = value.clone();
    std::thread::spawn(move || {
        cloned.fetch_add(1, Ordering::SeqCst);
    })
    .join()
    .unwrap();
    assert_eq!(value.load(Ordering::SeqCst), 1);
}

#[test]
fn shutdown_is_shared_idempotent_and_independent() {
    let handle = ShutdownHandle::new();
    let independent = ShutdownHandle::new();
    let cloned = handle.clone();
    assert!(!handle.is_requested());
    std::thread::spawn(move || {
        cloned.shutdown();
        cloned.shutdown();
    })
    .join()
    .unwrap();
    assert!(handle.is_requested());
    assert!(!independent.is_requested());
}
