use std::io;
use std::path::{Component, Path, PathBuf};

pub(crate) fn relative_workspace_path(root: &Path, requested: &Path) -> io::Result<PathBuf> {
    if requested.is_absolute() {
        return Err(path_escape());
    }
    let mut normalized = PathBuf::new();
    for component in requested.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(path_escape());
            }
        }
    }
    Ok(root.join(normalized))
}

pub(crate) fn contained_existing_path(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    let canonical_candidate = candidate.canonicalize()?;
    ensure_contained(&canonical_root, &canonical_candidate)?;
    Ok(canonical_candidate)
}

pub(crate) fn is_sensitive_workspace_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized
        .split('/')
        .filter(|component| !component.is_empty())
        .any(|component| {
            component == ".git"
                || component == ".ssh"
                || component == ".env"
                || component.starts_with(".env.")
                || component.contains("credentials")
                || component.contains("keychain")
                || component.contains("id_rsa")
                || component.ends_with(".pem")
                || component.ends_with(".key")
        })
}

fn ensure_contained(root: &Path, candidate: &Path) -> io::Result<()> {
    candidate
        .starts_with(root)
        .then_some(())
        .ok_or_else(path_escape)
}

fn path_escape() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "path_escape")
}
