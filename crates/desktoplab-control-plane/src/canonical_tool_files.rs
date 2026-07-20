use desktoplab_agent_engine::IterativeToolCall;
use desktoplab_redaction::redact_sensitive_with_status;
use desktoplab_tool_gateway::{
    FilesystemApproval, FilesystemMutationExecutor, FilesystemMutationOutcome,
    FilesystemPatchApproval, FilesystemPatchExecutor, FilesystemPatchOutcome,
    FilesystemPatchRequest, FilesystemToolExecutor, FilesystemToolOutcome,
};
use serde_json::{Value, json};

use crate::canonical_tool_executor::{
    CanonicalAgentToolExecutor, CanonicalExecutionApproval, optional_string, optional_usize,
    required_string, string_argument,
};
use crate::canonical_tool_search;

const OUTPUT_LIMIT: usize = 64 * 1024;

pub(crate) fn execute(
    executor: &CanonicalAgentToolExecutor,
    call: &IterativeToolCall,
) -> Result<Value, String> {
    match call.name() {
        "desktoplab.list_files" => {
            canonical_tool_search::list_files(executor, optional_string(call, "path"))
        }
        "desktoplab.search_text" => canonical_tool_search::search(
            executor,
            required_string(call, "query")?,
            optional_string(call, "path"),
            call.arguments()
                .get("regex")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            call.arguments()
                .get("caseSensitive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        "desktoplab.read_file" => read(
            executor,
            required_string(call, "path")?,
            optional_usize(call, "offset", 0, usize::MAX)?,
            optional_usize(call, "limit", 1000, 2000)?,
        ),
        "desktoplab.write_file" => write(
            executor,
            required_string(call, "path")?,
            string_argument(call, "content")?,
        ),
        "desktoplab.patch_file" => patch(
            executor,
            required_string(call, "path")?,
            required_string(call, "expected")?,
            string_argument(call, "replacement")?,
            call.arguments()
                .get("replaceAll")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        "desktoplab.create_directory" => mutate(executor, |mutator, approval| {
            Ok(mutator.create_directory(required_string(call, "path")?, approval))
        }),
        "desktoplab.move_path" => mutate(executor, |mutator, approval| {
            Ok(mutator.move_path(
                required_string(call, "source")?,
                required_string(call, "destination")?,
                approval,
            ))
        }),
        "desktoplab.delete_path" => mutate(executor, |mutator, approval| {
            Ok(mutator.delete_path(
                required_string(call, "path")?,
                call.arguments()
                    .get("recursive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                approval,
            ))
        }),
        _ => Err("unsupported_filesystem_tool".to_string()),
    }
}

fn mutate(
    executor: &CanonicalAgentToolExecutor,
    operation: impl FnOnce(
        &mut FilesystemMutationExecutor,
        FilesystemApproval,
    ) -> Result<FilesystemMutationOutcome, String>,
) -> Result<Value, String> {
    let mut mutator = FilesystemMutationExecutor::new(executor.root(), executor.policy());
    match operation(&mut mutator, filesystem_approval(executor.approval()))? {
        FilesystemMutationOutcome::Changed => Ok(json!({"changed":true})),
        FilesystemMutationOutcome::Unchanged => Ok(json!({"changed":false})),
        FilesystemMutationOutcome::ApprovalRequired => Err("approval_required".to_string()),
        FilesystemMutationOutcome::Denied => Err("approval_denied".to_string()),
        FilesystemMutationOutcome::Blocked(reason) => Err(reason),
    }
}

fn read(
    executor: &CanonicalAgentToolExecutor,
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let mut files = FilesystemToolExecutor::new(executor.root(), executor.policy());
    match files.read(path) {
        FilesystemToolOutcome::Read(text) => {
            let total_lines = text.lines().count();
            let page = text
                .lines()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>()
                .join("\n");
            let (text, byte_truncated) = bounded_redacted(&page);
            let returned_lines = page.lines().count();
            let end_line = offset.saturating_add(returned_lines).min(total_lines);
            Ok(json!({
                "path":path,
                "text":text,
                "startLine":if returned_lines == 0 { 0 } else { offset + 1 },
                "endLine":end_line,
                "totalLines":total_lines,
                "truncated":byte_truncated || offset > 0 || end_line < total_lines,
                "redacted":true
            }))
        }
        FilesystemToolOutcome::Blocked(reason) => Err(reason.to_string()),
        _ => Err("unexpected_filesystem_read_outcome".to_string()),
    }
}

fn write(
    executor: &CanonicalAgentToolExecutor,
    path: &str,
    content: &str,
) -> Result<Value, String> {
    let mut files = FilesystemToolExecutor::new(executor.root(), executor.policy());
    match files.write(path, content, filesystem_approval(executor.approval())) {
        FilesystemToolOutcome::Written => Ok(json!({"path":path,"changed":true})),
        FilesystemToolOutcome::Unchanged => Ok(json!({"path":path,"changed":false})),
        FilesystemToolOutcome::ApprovalRequired => Err("approval_required".to_string()),
        FilesystemToolOutcome::Denied => Err("approval_denied".to_string()),
        FilesystemToolOutcome::Blocked(reason) => Err(reason.to_string()),
        FilesystemToolOutcome::Read(_) => Err("unexpected_filesystem_write_outcome".to_string()),
    }
}

fn patch(
    executor: &CanonicalAgentToolExecutor,
    path: &str,
    expected: &str,
    replacement: &str,
    replace_all: bool,
) -> Result<Value, String> {
    let mut patcher = FilesystemPatchExecutor::new(executor.root(), executor.policy());
    let request = FilesystemPatchRequest::replace(path, expected, replacement);
    let request = if replace_all {
        request.with_replace_all()
    } else {
        request
    };
    match patcher.apply(request, patch_approval(executor.approval())) {
        FilesystemPatchOutcome::Patched(evidence) => Ok(json!({
            "path":path,"changed":true,
            "diff":format!("{}{}", evidence.before_diff(), evidence.after_diff())
        })),
        FilesystemPatchOutcome::ApprovalRequired => Err("approval_required".to_string()),
        FilesystemPatchOutcome::Denied => Err("approval_denied".to_string()),
        FilesystemPatchOutcome::Blocked(reason) => Err(reason.to_string()),
    }
}

fn bounded_redacted(value: &str) -> (String, bool) {
    let redacted = redact_sensitive_with_status(value);
    bounded_utf8(redacted.value(), OUTPUT_LIMIT)
}

fn bounded_utf8(value: &str, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value.to_string(), false);
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

fn filesystem_approval(value: CanonicalExecutionApproval) -> FilesystemApproval {
    match value {
        CanonicalExecutionApproval::Pending => FilesystemApproval::Pending,
        CanonicalExecutionApproval::Approved => FilesystemApproval::Approved,
        CanonicalExecutionApproval::Denied => FilesystemApproval::Denied,
    }
}

fn patch_approval(value: CanonicalExecutionApproval) -> FilesystemPatchApproval {
    match value {
        CanonicalExecutionApproval::Pending => FilesystemPatchApproval::Pending,
        CanonicalExecutionApproval::Approved => FilesystemPatchApproval::Approved,
        CanonicalExecutionApproval::Denied => FilesystemPatchApproval::Denied,
    }
}

#[cfg(test)]
mod tests {
    use super::bounded_utf8;

    #[test]
    fn file_output_limit_is_measured_in_bytes() {
        let (bounded, truncated) = bounded_utf8("ééé", 5);

        assert!(bounded.len() <= 5);
        assert!(truncated);
    }
}
