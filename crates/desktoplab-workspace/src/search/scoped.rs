use std::io;
use std::path::Path;

use super::{
    WorkspaceFileEntry, WorkspaceFileSafety, WorkspaceIndex, WorkspaceIndexLimits, WorkspaceSearch,
    WorkspaceSearchHit, WorkspaceSearchPattern, WorkspaceSearchReport, preview_lines,
};

impl WorkspaceSearch {
    pub fn search_file_with_options(
        &self,
        root: &Path,
        relative_file: &str,
        query: &str,
        regex: bool,
        case_sensitive: bool,
    ) -> io::Result<WorkspaceSearchReport> {
        self.search_filtered(root, query, regex, case_sensitive, Some(relative_file))
    }

    pub(super) fn search_filtered(
        &self,
        root: &Path,
        query: &str,
        regex: bool,
        case_sensitive: bool,
        relative_file: Option<&str>,
    ) -> io::Result<WorkspaceSearchReport> {
        let pattern = WorkspaceSearchPattern::new(query, regex, case_sensitive)?;
        let mut matches = Vec::new();
        let mut truncated = false;
        let entries = if let Some(file) = relative_file {
            let index = WorkspaceIndex::new(WorkspaceIndexLimits::new(
                self.limits.max_files,
                self.limits.max_bytes_per_file,
            ));
            index
                .lookup_file(root, Path::new(file))?
                .into_iter()
                .map(|entry| {
                    WorkspaceFileEntry::new(
                        entry.path(),
                        entry.size_bytes(),
                        entry.language(),
                        entry.safety(),
                    )
                })
                .collect()
        } else {
            self.list_files(root)?
        };
        for entry in entries {
            if entry.safety != WorkspaceFileSafety::Readable {
                continue;
            }
            let Some(text) = self.read_limited_text(root, entry.path())? else {
                continue;
            };
            let mut hits = preview_lines(&text, &pattern);
            if hits.is_empty() && pattern.matches(entry.path()) {
                hits.push((String::new(), None));
            }
            for (preview, line_number) in hits {
                matches.push(WorkspaceSearchHit {
                    path: entry.path.clone(),
                    preview,
                    line_number,
                });
                if matches.len() >= self.limits.max_matches {
                    truncated = true;
                    break;
                }
            }
            if truncated {
                break;
            }
        }
        Ok(WorkspaceSearchReport { matches, truncated })
    }
}
