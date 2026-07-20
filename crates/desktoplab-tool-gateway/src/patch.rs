use std::path::Path;

use desktoplab_policy::{Action, PolicyEngine};

use crate::{
    ToolGateway, ToolIntent, ToolOutcome, WorkspaceRoot, path_security::is_sensitive_workspace_path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilesystemPatchApproval {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesystemPatchRequest {
    path: String,
    expected: String,
    replacement: String,
    replace_all: bool,
}

impl FilesystemPatchRequest {
    #[must_use]
    pub fn replace(
        path: impl Into<String>,
        expected: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            expected: expected.into(),
            replacement: replacement.into(),
            replace_all: false,
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn expected(&self) -> &str {
        &self.expected
    }

    #[must_use]
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    #[must_use]
    pub fn with_replace_all(mut self) -> Self {
        self.replace_all = true;
        self
    }

    #[must_use]
    pub fn replace_all(&self) -> bool {
        self.replace_all
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesystemPatchEvidence {
    before_diff: String,
    after_diff: String,
}

impl FilesystemPatchEvidence {
    #[must_use]
    pub fn before_diff(&self) -> &str {
        &self.before_diff
    }

    #[must_use]
    pub fn after_diff(&self) -> &str {
        &self.after_diff
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilesystemPatchOutcome {
    Patched(FilesystemPatchEvidence),
    ApprovalRequired,
    Denied,
    Blocked(&'static str),
}

pub struct FilesystemPatchExecutor {
    root: Option<WorkspaceRoot>,
    gateway: ToolGateway,
}

impl FilesystemPatchExecutor {
    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine) -> Self {
        Self {
            root: WorkspaceRoot::open(root).ok(),
            gateway: ToolGateway::new(policy),
        }
    }

    pub fn apply(
        &mut self,
        request: FilesystemPatchRequest,
        approval: FilesystemPatchApproval,
    ) -> FilesystemPatchOutcome {
        if is_sensitive_workspace_path(request.path()) {
            return FilesystemPatchOutcome::Blocked("protected_path");
        }
        let Some(root) = &self.root else {
            return FilesystemPatchOutcome::Blocked("path_escape");
        };
        match self
            .gateway
            .authorize(ToolIntent::filesystem_patch(request.path()))
        {
            ToolOutcome::Blocked(_) => return FilesystemPatchOutcome::Blocked("local_only_path"),
            ToolOutcome::Allowed(Action::FilesystemWrite) => {}
            ToolOutcome::ApprovalRequired(Action::FilesystemWrite) => match approval {
                FilesystemPatchApproval::Pending => {
                    return FilesystemPatchOutcome::ApprovalRequired;
                }
                FilesystemPatchApproval::Denied => return FilesystemPatchOutcome::Denied,
                FilesystemPatchApproval::Approved => {}
            },
            ToolOutcome::Allowed(_) | ToolOutcome::ApprovalRequired(_) => {
                return FilesystemPatchOutcome::Blocked("unexpected_action");
            }
        }
        let Ok(mut file) = root.open_update(request.path()) else {
            return FilesystemPatchOutcome::Blocked("path_escape");
        };
        let Ok(current) = file.read_text() else {
            return FilesystemPatchOutcome::Blocked("read_failed");
        };
        let updated = match replace_preserving_line_endings(
            &current,
            request.expected(),
            request.replacement(),
            request.replace_all(),
        ) {
            PatchReplacement::Updated(updated) => updated,
            PatchReplacement::Missing => {
                return FilesystemPatchOutcome::Blocked("patch_conflict");
            }
            PatchReplacement::Ambiguous => {
                return FilesystemPatchOutcome::Blocked("patch_ambiguous");
            }
        };
        if file.replace_text(&updated).is_err() {
            return FilesystemPatchOutcome::Blocked("write_failed");
        }
        FilesystemPatchOutcome::Patched(FilesystemPatchEvidence {
            before_diff: simple_diff(request.path(), request.expected(), ""),
            after_diff: simple_diff(request.path(), "", request.replacement()),
        })
    }
}

fn replace_preserving_line_endings(
    current: &str,
    expected: &str,
    replacement: &str,
    replace_all: bool,
) -> PatchReplacement {
    let line_ending = if current.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let expected = if current.contains(expected) {
        expected.to_string()
    } else {
        with_line_ending(expected, line_ending)
    };
    if expected.is_empty() || !current.contains(&expected) {
        return PatchReplacement::Missing;
    }
    if !replace_all && current.match_indices(&expected).nth(1).is_some() {
        return PatchReplacement::Ambiguous;
    }
    let replacement = with_line_ending(replacement, line_ending);
    let updated = if replace_all {
        current.replace(&expected, &replacement)
    } else {
        current.replacen(&expected, &replacement, 1)
    };
    PatchReplacement::Updated(updated)
}

enum PatchReplacement {
    Updated(String),
    Missing,
    Ambiguous,
}

fn with_line_ending(text: &str, line_ending: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', line_ending)
}

fn simple_diff(path: &str, removed: &str, added: &str) -> String {
    let mut diff = format!("diff --git a/{path} b/{path}\n");
    for line in removed.lines() {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in added.lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff
}
