use berserk_cache::{Cache, MemoryCache, Namespaced};
use std::{sync::Arc, thread, time::Duration};

#[test]
fn values_can_be_stored_replaced_and_forgotten() {
    let cache = MemoryCache::new(2, 16).unwrap();
    cache.put("name", b"first".to_vec(), None).unwrap();
    cache.put("name", b"second".to_vec(), None).unwrap();
    assert_eq!(cache.get("name").unwrap(), Some(b"second".to_vec()));
    assert!(cache.forget("name").unwrap());
    assert!(!cache.forget("name").unwrap());
}

#[test]
fn add_is_atomic_and_does_not_replace() {
    let cache = MemoryCache::new(2, 16).unwrap();
    assert!(cache.add("key", b"one".to_vec(), None).unwrap());
    assert!(!cache.add("key", b"two".to_vec(), None).unwrap());
    assert_eq!(cache.get("key").unwrap(), Some(b"one".to_vec()));
}

#[test]
fn expired_values_are_removed() {
    let cache = MemoryCache::new(2, 16).unwrap();
    cache
        .put("short", b"value".to_vec(), Some(Duration::ZERO))
        .unwrap();
    assert_eq!(cache.get("short").unwrap(), None);
    assert!(cache.is_empty().unwrap());
}

#[test]
fn least_recently_used_value_is_evicted() {
    let cache = MemoryCache::new(2, 16).unwrap();
    cache.put("a", vec![1], None).unwrap();
    cache.put("b", vec![2], None).unwrap();
    cache.get("a").unwrap();
    cache.put("c", vec![3], None).unwrap();
    assert_eq!(cache.get("b").unwrap(), None);
    assert_eq!(cache.get("a").unwrap(), Some(vec![1]));
}

#[test]
fn increments_are_atomic_across_threads() {
    let cache = Arc::new(MemoryCache::new(2, 32).unwrap());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || {
                for _ in 0..100 {
                    cache.increment("count", 1, None).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(cache.get("count").unwrap(), Some(b"400".to_vec()));
}

#[test]
fn namespaces_isolate_the_same_logical_key() {
    let cache = Arc::new(MemoryCache::new(4, 16).unwrap());
    let first = Namespaced::new("first", Arc::clone(&cache)).unwrap();
    let second = Namespaced::new("second", Arc::clone(&cache)).unwrap();
    first.put("key", vec![1], None).unwrap();
    second.put("key", vec![2], None).unwrap();
    assert_eq!(first.get("key").unwrap(), Some(vec![1]));
    assert_eq!(second.get("key").unwrap(), Some(vec![2]));
}
