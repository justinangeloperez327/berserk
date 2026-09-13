use framework_cache::{Cache, MemoryCache};

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
