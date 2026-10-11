use berserk_cache::{Cache, MemoryCache};

#[test]
fn cache_debug_output_redacts_keys_and_values() {
    let cache = MemoryCache::new(8, 1024).unwrap();
    cache
        .put("secret-key", b"secret-value".to_vec(), None)
        .unwrap();
    let debug = format!("{cache:?}");
    assert!(!debug.contains("secret-key"));
    assert!(!debug.contains("secret-value"));
    assert!(debug.contains("entry_count"));
}

#[test]
fn counters_respect_value_limits_without_overwriting_or_evicting() {
    let cache = MemoryCache::new(1, 1).unwrap();
    cache.put("counter", b"9".to_vec(), None).unwrap();
    assert!(cache.increment("counter", 1, None).is_err());
    assert_eq!(cache.get("counter").unwrap(), Some(b"9".to_vec()));
    assert!(cache.increment("oversized", 1000, None).is_err());
    assert!(cache.get("oversized").unwrap().is_none());
    assert_eq!(cache.get("counter").unwrap(), Some(b"9".to_vec()));
}
