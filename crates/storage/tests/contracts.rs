use framework_storage::{ErrorKind, LocalStorage, MemoryStorage, Storage, StoragePath};
use std::{
    fs,
    io::{Cursor, Read},
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn paths_reject_traversal_and_ambiguous_forms() {
    for path in ["", "/root", "../secret", "a/../b", "a//b", "a\\b"] {
        assert_eq!(
            StoragePath::new(path).unwrap_err().kind(),
            ErrorKind::InvalidPath
        );
    }
}

#[test]
fn memory_storage_supports_the_object_lifecycle() {
    let storage = MemoryStorage::new(2, 16).unwrap();
    let path = StoragePath::new("avatars/one.txt").unwrap();
    let metadata = storage.put(&path, &mut Cursor::new(b"hello")).unwrap();
    assert_eq!(metadata.bytes, 5);
    let mut body = Vec::new();
    storage.read(&path).unwrap().read_to_end(&mut body).unwrap();
    assert_eq!(body, b"hello");
    assert_eq!(
        storage
            .list(Some(&StoragePath::new("avatars").unwrap()))
            .unwrap()
            .len(),
        1
    );
    assert!(storage.delete(&path).unwrap());
    assert!(!storage.exists(&path).unwrap());
}

#[test]
fn failed_oversized_memory_write_preserves_existing_value() {
    let storage = MemoryStorage::new(1, 4).unwrap();
    let path = StoragePath::new("item").unwrap();
    storage.put(&path, &mut Cursor::new(b"old")).unwrap();
    assert_eq!(
        storage
            .put(&path, &mut Cursor::new(b"oversized"))
            .unwrap_err()
            .kind(),
        ErrorKind::TooLarge
    );
    let mut value = String::new();
    storage
        .read(&path)
        .unwrap()
        .read_to_string(&mut value)
        .unwrap();
    assert_eq!(value, "old");
}

#[test]
fn local_storage_streams_nested_files_and_replaces_objects() {
    let temporary = TemporaryDirectory::new();
    let storage = LocalStorage::new(temporary.path(), 16, 8).unwrap();
    let path = StoragePath::new("nested/item.txt").unwrap();
    storage.put(&path, &mut Cursor::new(b"first")).unwrap();
    storage.put(&path, &mut Cursor::new(b"next")).unwrap();
    let mut body = String::new();
    storage
        .read(&path)
        .unwrap()
        .read_to_string(&mut body)
        .unwrap();
    assert_eq!(body, "next");
    assert_eq!(storage.list(None).unwrap()[0].path, path);
}

#[test]
fn failed_oversized_local_write_preserves_existing_object() {
    let temporary = TemporaryDirectory::new();
    let storage = LocalStorage::new(temporary.path(), 4, 8).unwrap();
    let path = StoragePath::new("item").unwrap();
    storage.put(&path, &mut Cursor::new(b"old")).unwrap();
    assert_eq!(
        storage
            .put(&path, &mut Cursor::new(b"too large"))
            .unwrap_err()
            .kind(),
        ErrorKind::TooLarge
    );
    let mut body = String::new();
    storage
        .read(&path)
        .unwrap()
        .read_to_string(&mut body)
        .unwrap();
    assert_eq!(body, "old");
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("framework-storage-{}-{nonce}", process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &PathBuf {
        &self.0
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
