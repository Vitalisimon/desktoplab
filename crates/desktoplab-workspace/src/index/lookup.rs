use std::io;
use std::path::{Component, Path};

use super::{
    IgnoreRules, WorkspaceIndex, WorkspaceIndexEntry, index_entry, normalize,
    should_skip_index_path,
};
use crate::inspection::is_protected_path;

impl WorkspaceIndex {
    pub(crate) fn lookup_file(
        &self,
        root: &Path,
        relative: &Path,
    ) -> io::Result<Option<WorkspaceIndexEntry>> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid relative path",
            ));
        }
        let text = normalize(relative);
        if should_skip_index_path(&text) || IgnoreRules::load(root).matches(&text) {
            return Ok(None);
        }
        let canonical_root = root.canonicalize()?;
        let candidate = root.join(relative).canonicalize()?;
        if !candidate.starts_with(&canonical_root) || !candidate.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid file scope",
            ));
        }
        let resolved = normalize(
            candidate
                .strip_prefix(&canonical_root)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid file scope"))?,
        );
        if is_protected_path(&resolved)
            || should_skip_index_path(&resolved)
            || IgnoreRules::load(root).matches(&resolved)
        {
            return Ok(None);
        }
        index_entry(root, relative, &self.limits)
    }
}
