use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use fs2::FileExt;
use tempfile::NamedTempFile;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateFileMode {
    Inherit,
    OwnerOnly,
}

#[derive(Debug)]
pub enum LockError {
    Busy,
    Io(std::io::Error),
}

impl From<std::io::Error> for LockError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct CrossProcessLock {
    file: File,
    path: PathBuf,
    registry_key: PathBuf,
}

impl CrossProcessLock {
    pub fn acquire(path: &Path, attempts: u8, retry_delay: Duration) -> Result<Self, LockError> {
        let registry_key = absolute_path(path)?;
        if !reserve_process_lock(&registry_key) {
            return Err(LockError::Busy);
        }
        match Self::acquire_reserved(path, attempts, retry_delay, registry_key.clone()) {
            Ok(lock) => Ok(lock),
            Err(error) => {
                release_process_lock(&registry_key);
                Err(error)
            }
        }
    }

    fn acquire_reserved(
        path: &Path,
        attempts: u8,
        retry_delay: Duration,
        registry_key: PathBuf,
    ) -> Result<Self, LockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = private_lock_file(path)?;
        for attempt in 0..attempts.max(1) {
            match file.try_lock_exclusive() {
                Ok(()) => {
                    file.set_len(0)?;
                    file.seek(SeekFrom::Start(0))?;
                    write!(file, "pid={}\n", std::process::id())?;
                    file.sync_data()?;
                    return Ok(Self {
                        file,
                        path: path.to_path_buf(),
                        registry_key,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if attempt + 1 < attempts.max(1) {
                        thread::sleep(retry_delay);
                    }
                }
                Err(error) => return Err(LockError::Io(error)),
            }
        }
        Err(LockError::Busy)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CrossProcessLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        release_process_lock(&self.registry_key);
    }
}

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn process_locks() -> &'static Mutex<HashSet<PathBuf>> {
    static LOCKS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn reserve_process_lock(path: &Path) -> bool {
    process_locks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(path.to_path_buf())
}

fn release_process_lock(path: &Path) {
    process_locks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(path);
}

pub struct AtomicFileStore;

impl AtomicFileStore {
    pub fn replace(path: &Path, bytes: &[u8], mode: PrivateFileMode) -> Result<(), std::io::Error> {
        Self::replace_with(path, mode, |file| file.write_all(bytes))
    }

    pub fn replace_with(
        path: &Path,
        mode: PrivateFileMode,
        writer: impl FnOnce(&mut File) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("missing_parent"))?;
        std::fs::create_dir_all(parent)?;
        let lock_path = lock_path(path);
        let _lock = CrossProcessLock::acquire(&lock_path, 8, Duration::from_millis(10))
            .map_err(lock_io_error)?;
        let inherited = std::fs::metadata(path)
            .ok()
            .map(|metadata| metadata.permissions());
        let mut temporary = NamedTempFile::new_in(parent)?;
        if let Some(permissions) = inherited {
            temporary.as_file().set_permissions(permissions)?;
        }
        set_private_permissions(temporary.path(), temporary.as_file(), mode)?;
        writer(temporary.as_file_mut())?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        sync_parent(parent)?;
        Ok(())
    }
}

fn lock_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state");
    path.with_file_name(format!(".{name}.lock"))
}

fn private_lock_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(unix)]
fn set_private_permissions(
    _path: &Path,
    file: &File,
    mode: PrivateFileMode,
) -> std::io::Result<()> {
    if mode == PrivateFileMode::OwnerOnly {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(windows)]
fn set_private_permissions(
    path: &Path,
    _file: &File,
    mode: PrivateFileMode,
) -> std::io::Result<()> {
    if mode == PrivateFileMode::OwnerOnly {
        crate::windows_acl::restrict_to_current_user(path)?;
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn set_private_permissions(
    _path: &Path,
    _file: &File,
    _mode: PrivateFileMode,
) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> std::io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

fn lock_io_error(error: LockError) -> std::io::Error {
    match error {
        LockError::Busy => std::io::Error::new(std::io::ErrorKind::WouldBlock, "state_lock_busy"),
        LockError::Io(error) => error,
    }
}
