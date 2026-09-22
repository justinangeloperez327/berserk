use crate::{ErrorKind, ObjectMetadata, Result, Storage, StorageError, StoragePath};
use rand_core::{OsRng, RngCore};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub struct LocalStorage {
    root: PathBuf,
    max_object_bytes: usize,
    max_list_results: usize,
}

impl LocalStorage {
    pub fn new(
        root: impl AsRef<Path>,
        max_object_bytes: usize,
        max_list_results: usize,
    ) -> Result<Self> {
        if max_object_bytes == 0 || max_list_results == 0 {
            return Err(StorageError::new(
                ErrorKind::Capacity,
                "storage limits must be greater than zero",
            ));
        }
        fs::create_dir_all(root.as_ref()).map_err(StorageError::io)?;
        let root = fs::canonicalize(root).map_err(StorageError::io)?;
        Ok(Self {
            root,
            max_object_bytes,
            max_list_results,
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn resolve(&self, path: &StoragePath) -> Result<PathBuf> {
        let mut target = self.root.clone();
        for part in path.as_str().split('/') {
            target.push(part);
            if let Ok(metadata) = fs::symlink_metadata(&target) {
                if metadata.file_type().is_symlink() {
                    return Err(StorageError::new(
                        ErrorKind::InvalidPath,
                        "symbolic links are not allowed inside local storage",
                    ));
                }
            }
        }
        Ok(target)
    }
    fn object_metadata(&self, path: StoragePath, metadata: fs::Metadata) -> Result<ObjectMetadata> {
        if !metadata.is_file() {
            return Err(StorageError::new(
                ErrorKind::InvalidPath,
                "storage object is not a regular file",
            ));
        }
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs());
        Ok(ObjectMetadata {
            path,
            bytes: metadata.len(),
            modified_at,
        })
    }
}

impl Storage for LocalStorage {
    fn put(&self, path: &StoragePath, source: &mut dyn Read) -> Result<ObjectMetadata> {
        let target = self.resolve(path)?;
        let parent = target.parent().ok_or_else(|| {
            StorageError::new(ErrorKind::InvalidPath, "storage path has no parent")
        })?;
        fs::create_dir_all(parent).map_err(StorageError::io)?;
        self.resolve(path)?;
        let temp = temporary_path(parent)?;
        let mut cleanup = TempFile {
            path: temp.clone(),
            committed: false,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(StorageError::io)?;
        let mut total = 0_usize;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = source.read(&mut buffer).map_err(StorageError::io)?;
            if read == 0 {
                break;
            }
            total = total.checked_add(read).ok_or_else(|| {
                StorageError::new(
                    ErrorKind::TooLarge,
                    "stored object exceeds configured byte limit",
                )
            })?;
            if total > self.max_object_bytes {
                return Err(StorageError::new(
                    ErrorKind::TooLarge,
                    "stored object exceeds configured byte limit",
                ));
            }
            file.write_all(&buffer[..read]).map_err(StorageError::io)?;
        }
        file.sync_all().map_err(StorageError::io)?;
        drop(file);
        replace_file(&temp, &target)?;
        cleanup.committed = true;
        self.metadata(path)?.ok_or_else(|| {
            StorageError::new(ErrorKind::Io, "stored object disappeared after write")
        })
    }
    fn read(&self, path: &StoragePath) -> Result<Box<dyn Read + Send>> {
        let target = self.resolve(path)?;
        let metadata = fs::symlink_metadata(&target).map_err(StorageError::io)?;
        if !metadata.is_file() {
            return Err(StorageError::new(
                ErrorKind::InvalidPath,
                "storage object is not a regular file",
            ));
        }
        Ok(Box::new(File::open(target).map_err(StorageError::io)?))
    }
    fn metadata(&self, path: &StoragePath) -> Result<Option<ObjectMetadata>> {
        let target = self.resolve(path)?;
        match fs::symlink_metadata(target) {
            Ok(metadata) => self.object_metadata(path.clone(), metadata).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::io(error)),
        }
    }
    fn delete(&self, path: &StoragePath) -> Result<bool> {
        let target = self.resolve(path)?;
        match fs::remove_file(target) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(StorageError::io(error)),
        }
    }
    fn list(&self, prefix: Option<&StoragePath>) -> Result<Vec<ObjectMetadata>> {
        let start = match prefix {
            Some(path) => self.resolve(path)?,
            None => self.root.clone(),
        };
        if !start.exists() {
            return Ok(Vec::new());
        }
        let start_metadata = fs::symlink_metadata(&start).map_err(StorageError::io)?;
        if start_metadata.file_type().is_symlink() {
            return Err(StorageError::new(
                ErrorKind::InvalidPath,
                "symbolic links are not allowed inside local storage",
            ));
        }
        if start_metadata.is_file() {
            let path = prefix.cloned().ok_or_else(|| {
                StorageError::new(ErrorKind::InvalidPath, "storage root cannot be a file")
            })?;
            return self
                .object_metadata(path, start_metadata)
                .map(|metadata| vec![metadata]);
        }
        let mut files = Vec::new();
        let mut pending = vec![start];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).map_err(StorageError::io)? {
                let entry = entry.map_err(StorageError::io)?;
                let metadata = fs::symlink_metadata(entry.path()).map_err(StorageError::io)?;
                if metadata.file_type().is_symlink() {
                    return Err(StorageError::new(
                        ErrorKind::InvalidPath,
                        "symbolic links are not allowed inside local storage",
                    ));
                }
                if metadata.is_dir() {
                    pending.push(entry.path());
                    continue;
                }
                let relative = entry
                    .path()
                    .strip_prefix(&self.root)
                    .map_err(|_| {
                        StorageError::new(ErrorKind::InvalidPath, "object escaped storage root")
                    })?
                    .to_string_lossy()
                    .replace('\\', "/");
                let path = StoragePath::new(relative)?;
                files.push(self.object_metadata(path, metadata)?);
                if files.len() > self.max_list_results {
                    return Err(StorageError::new(
                        ErrorKind::Capacity,
                        "storage listing exceeds configured result limit",
                    ));
                }
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(files)
    }
}

struct TempFile {
    path: PathBuf,
    committed: bool,
}
impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn temporary_path(parent: &Path) -> Result<PathBuf> {
    for _ in 0..8 {
        let mut random = [0_u8; 8];
        OsRng.try_fill_bytes(&mut random).map_err(|_| {
            StorageError::new(
                ErrorKind::Io,
                "operating system randomness unavailable for temporary storage path",
            )
        })?;
        let path = parent.join(format!(".framework-{}.tmp", hex(&random)));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(StorageError::new(
        ErrorKind::AlreadyExists,
        "could not allocate a unique temporary storage path",
    ))
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    match fs::rename(source, target) {
        Ok(()) => return Ok(()),
        Err(error)
            if error.kind() != std::io::ErrorKind::AlreadyExists
                && error.kind() != std::io::ErrorKind::PermissionDenied =>
        {
            return Err(StorageError::io(error))
        }
        Err(_) => {}
    }
    if !target.exists() {
        return fs::rename(source, target).map_err(StorageError::io);
    }
    let parent = target
        .parent()
        .ok_or_else(|| StorageError::new(ErrorKind::InvalidPath, "storage path has no parent"))?;
    let backup = temporary_path(parent)?;
    fs::rename(target, &backup).map_err(StorageError::io)?;
    match fs::rename(source, target) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, target);
            Err(StorageError::io(error))
        }
    }
}
