use std::fmt;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_expansion_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 2_000,
            max_total_bytes: 512 * 1024 * 1024,
            max_entry_bytes: 64 * 1024 * 1024,
            max_expansion_ratio: 200,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveReport {
    pub entries: usize,
    pub total_bytes: u64,
}

#[derive(Debug)]
pub enum ArchiveImportError {
    InvalidEntry,
    LinkEntry,
    LimitExceeded,
    DestinationExists,
    Io(String),
}

impl fmt::Display for ArchiveImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

pub fn import_archive<R: Read + Seek>(
    reader: R,
    format: ArchiveFormat,
    destination: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveReport, ArchiveImportError> {
    if destination.exists() {
        return Err(ArchiveImportError::DestinationExists);
    }
    let parent = destination
        .parent()
        .ok_or(ArchiveImportError::InvalidEntry)?;
    std::fs::create_dir_all(parent).map_err(io_error)?;
    let mut staging = tempfile::TempDir::new_in(parent).map_err(io_error)?;
    let report = match format {
        ArchiveFormat::Zip => extract_zip(reader, staging.path(), limits)?,
        ArchiveFormat::Tar => extract_tar(reader, staging.path(), limits)?,
    };
    std::fs::rename(staging.path(), destination).map_err(io_error)?;
    staging.disable_cleanup(true);
    Ok(report)
}

fn extract_zip<R: Read + Seek>(
    reader: R,
    root: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveReport, ArchiveImportError> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|error| ArchiveImportError::Io(error.to_string()))?;
    if archive.len() > limits.max_entries {
        return Err(ArchiveImportError::LimitExceeded);
    }
    let mut total = 0_u64;
    let mut compressed = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ArchiveImportError::Io(error.to_string()))?;
        let relative = safe_path(entry.name())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ArchiveImportError::LinkEntry);
        }
        let size = entry.size();
        total = bounded_total(total, size, limits)?;
        compressed = compressed.saturating_add(entry.compressed_size());
        if total > compressed.max(1).saturating_mul(limits.max_expansion_ratio) {
            return Err(ArchiveImportError::LimitExceeded);
        }
        let target = root.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(target).map_err(io_error)?;
        } else {
            write_entry(&mut entry, &target, size)?;
        }
    }
    Ok(ArchiveReport {
        entries: archive.len(),
        total_bytes: total,
    })
}

fn extract_tar<R: Read>(
    reader: R,
    root: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveReport, ArchiveImportError> {
    let mut archive = tar::Archive::new(reader);
    let mut entries = 0_usize;
    let mut total = 0_u64;
    for item in archive.entries().map_err(io_error)? {
        let mut entry = item.map_err(io_error)?;
        entries += 1;
        if entries > limits.max_entries {
            return Err(ArchiveImportError::LimitExceeded);
        }
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            return Err(ArchiveImportError::LinkEntry);
        }
        let size = entry.size();
        total = bounded_total(total, size, limits)?;
        let relative = safe_path(&entry.path().map_err(io_error)?.to_string_lossy())?;
        let target = root.join(relative);
        if kind.is_dir() {
            std::fs::create_dir_all(target).map_err(io_error)?;
        } else if kind.is_file() {
            write_entry(&mut entry, &target, size)?;
        } else {
            return Err(ArchiveImportError::InvalidEntry);
        }
    }
    Ok(ArchiveReport {
        entries,
        total_bytes: total,
    })
}

fn bounded_total(
    current: u64,
    size: u64,
    limits: ArchiveLimits,
) -> Result<u64, ArchiveImportError> {
    if size > limits.max_entry_bytes {
        return Err(ArchiveImportError::LimitExceeded);
    }
    let total = current
        .checked_add(size)
        .ok_or(ArchiveImportError::LimitExceeded)?;
    (total <= limits.max_total_bytes)
        .then_some(total)
        .ok_or(ArchiveImportError::LimitExceeded)
}

fn safe_path(value: &str) -> Result<PathBuf, ArchiveImportError> {
    let path = Path::new(value);
    if path.is_absolute() || path.as_os_str().is_empty() {
        return Err(ArchiveImportError::InvalidEntry);
    }
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then(|| path.to_path_buf())
        .ok_or(ArchiveImportError::InvalidEntry)
}

fn write_entry(
    reader: &mut impl Read,
    target: &Path,
    expected: u64,
) -> Result<(), ArchiveImportError> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(io_error)?;
    }
    let mut file = File::create(target).map_err(io_error)?;
    let copied = std::io::copy(&mut reader.take(expected), &mut file).map_err(io_error)?;
    if copied != expected {
        return Err(ArchiveImportError::Io(
            "truncated_archive_entry".to_string(),
        ));
    }
    file.sync_all().map_err(io_error)
}

fn io_error(error: impl fmt::Display) -> ArchiveImportError {
    ArchiveImportError::Io(error.to_string())
}
