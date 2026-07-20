use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use cap_fs_ext::{FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};

#[derive(Debug)]
pub enum WorkspaceRootError {
    PathEscape,
    LinkAlias,
    NotRegularFile,
    Io(std::io::Error),
}

impl fmt::Display for WorkspaceRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathEscape => write!(formatter, "path_escape"),
            Self::LinkAlias => write!(formatter, "link_alias"),
            Self::NotRegularFile => write!(formatter, "not_regular_file"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<std::io::Error> for WorkspaceRootError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct WorkspaceRoot {
    dir: Dir,
    display_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspacePathState {
    Missing,
    File,
    Directory,
    Other,
}

impl WorkspaceRoot {
    pub fn open(root: &Path) -> Result<Self, WorkspaceRootError> {
        let display_path = root.canonicalize()?;
        let dir = Dir::open_ambient_dir(&display_path, ambient_authority())?;
        Ok(Self { dir, display_path })
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub fn read_text(&self, requested: &str) -> Result<String, WorkspaceRootError> {
        let mut file = self.open_existing(requested, false)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        Ok(text)
    }

    pub fn path_state(&self, requested: &str) -> Result<WorkspacePathState, WorkspaceRootError> {
        let relative = relative_path(requested)?;
        let metadata = match self.dir.symlink_metadata(&relative) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(WorkspacePathState::Missing);
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(WorkspaceRootError::LinkAlias);
        }
        Ok(if metadata.is_file() {
            WorkspacePathState::File
        } else if metadata.is_dir() {
            WorkspacePathState::Directory
        } else {
            WorkspacePathState::Other
        })
    }

    pub fn write_text(&self, requested: &str, contents: &str) -> Result<bool, WorkspaceRootError> {
        let relative = relative_path(requested)?;
        if let Some(parent) = relative
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            self.dir.create_dir_all(parent)?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        options.follow(FollowSymlinks::No);
        let mut file = self.dir.open_with(&relative, &options)?;
        ensure_single_regular_file(&file)?;
        let mut existing = Vec::new();
        file.read_to_end(&mut existing)?;
        if existing == contents.as_bytes() {
            return Ok(false);
        }
        replace_open_file(&mut file, contents.as_bytes())?;
        Ok(true)
    }

    pub(crate) fn create_directory(&self, requested: &str) -> Result<bool, WorkspaceRootError> {
        let relative = relative_path(requested)?;
        if self.dir.symlink_metadata(&relative).is_ok() {
            let metadata = self.dir.metadata(&relative)?;
            return if metadata.is_dir() {
                Ok(false)
            } else {
                Err(WorkspaceRootError::NotRegularFile)
            };
        }
        self.dir.create_dir_all(relative)?;
        Ok(true)
    }

    pub(crate) fn move_path(
        &self,
        source: &str,
        destination: &str,
    ) -> Result<(), WorkspaceRootError> {
        let source = relative_path(source)?;
        let destination = relative_path(destination)?;
        reject_link(&self.dir, &source)?;
        if self.dir.symlink_metadata(&destination).is_ok() {
            return Err(WorkspaceRootError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "destination_exists",
            )));
        }
        if let Some(parent) = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            self.dir.create_dir_all(parent)?;
        }
        self.dir.rename(source, &self.dir, destination)?;
        Ok(())
    }

    pub(crate) fn delete_path(
        &self,
        requested: &str,
        recursive: bool,
    ) -> Result<(), WorkspaceRootError> {
        let relative = relative_path(requested)?;
        reject_link(&self.dir, &relative)?;
        let metadata = self.dir.metadata(&relative)?;
        if metadata.is_dir() {
            if recursive {
                self.dir.remove_dir_all(relative)?;
            } else {
                self.dir.remove_dir(relative)?;
            }
        } else if metadata.is_file() {
            self.dir.remove_file(relative)?;
        } else {
            return Err(WorkspaceRootError::NotRegularFile);
        }
        Ok(())
    }

    pub(crate) fn open_update(
        &self,
        requested: &str,
    ) -> Result<WorkspaceRootFile, WorkspaceRootError> {
        self.open_existing(requested, true)
            .map(|file| WorkspaceRootFile { file })
    }

    fn open_existing(&self, requested: &str, write: bool) -> Result<File, WorkspaceRootError> {
        let relative = relative_path(requested)?;
        if self
            .dir
            .symlink_metadata(&relative)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(WorkspaceRootError::LinkAlias);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(write);
        options.follow(FollowSymlinks::No);
        let file = self.dir.open_with(relative, &options)?;
        ensure_single_regular_file(&file)?;
        Ok(file)
    }
}

pub(crate) struct WorkspaceRootFile {
    file: File,
}

impl WorkspaceRootFile {
    pub fn read_text(&mut self) -> Result<String, WorkspaceRootError> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut text = String::new();
        self.file.read_to_string(&mut text)?;
        Ok(text)
    }

    pub fn replace_text(&mut self, contents: &str) -> Result<(), WorkspaceRootError> {
        ensure_single_regular_file(&self.file)?;
        replace_open_file(&mut self.file, contents.as_bytes()).map_err(Into::into)
    }
}

fn relative_path(requested: &str) -> Result<PathBuf, WorkspaceRootError> {
    let path = Path::new(requested);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(WorkspaceRootError::PathEscape);
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => return Err(WorkspaceRootError::PathEscape),
        }
    }
    Ok(normalized)
}

fn ensure_single_regular_file(file: &File) -> Result<(), WorkspaceRootError> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(WorkspaceRootError::NotRegularFile);
    }
    if metadata.nlink() != 1 {
        return Err(WorkspaceRootError::LinkAlias);
    }
    Ok(())
}

fn reject_link(dir: &Dir, relative: &Path) -> Result<(), WorkspaceRootError> {
    if dir
        .symlink_metadata(relative)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(WorkspaceRootError::LinkAlias);
    }
    Ok(())
}

fn replace_open_file(file: &mut File, contents: &[u8]) -> std::io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(contents)?;
    file.sync_data()
}
