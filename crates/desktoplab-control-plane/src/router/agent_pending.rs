#[cfg(debug_assertions)]
use std::path::{Component, Path};

use desktoplab_agent_engine::IterativeToolCall;
use desktoplab_tool_gateway::{TerminalRiskClass, ToolIntent};
use serde_json::{Value, json};

use super::payload_hash::stable_payload_hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PendingAgentActionState {
    Pending,
    Applying,
    Applied,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingAgentAction {
    approval_id: String,
    session_id: String,
    tool: ToolIntent,
    content: Option<String>,
    payload_hash: String,
    state: PendingAgentActionState,
    readback_after_write: bool,
    checkpoint_id: Option<String>,
    checkpoint_status: Option<String>,
    approved_change_fingerprint: Option<String>,
    iterative_call: Option<IterativeToolCall>,
}

impl PendingAgentAction {
    #[must_use]
    pub(crate) fn new(
        approval_id: impl Into<String>,
        session_id: impl Into<String>,
        tool: ToolIntent,
        content: Option<String>,
        readback_after_write: bool,
    ) -> Self {
        let approval_id = approval_id.into();
        let session_id = session_id.into();
        let payload_hash = stable_payload_hash(&json!({
            "sessionId":session_id,
            "tool":tool_payload(&tool),
            "content":content,
            "readbackAfterWrite":readback_after_write,
            "checkpointId":null,
            "checkpointStatus":null,
            "approvedChangeFingerprint":null
            ,"iterativeCall":null
        }));
        Self {
            approval_id,
            session_id,
            tool,
            content,
            payload_hash,
            state: PendingAgentActionState::Pending,
            readback_after_write,
            checkpoint_id: None,
            checkpoint_status: None,
            approved_change_fingerprint: None,
            iterative_call: None,
        }
    }

    #[must_use]
    pub(crate) fn with_checkpoint(
        mut self,
        checkpoint_id: impl Into<String>,
        checkpoint_status: impl Into<String>,
    ) -> Self {
        self.checkpoint_id = Some(checkpoint_id.into());
        self.checkpoint_status = Some(checkpoint_status.into());
        self.payload_hash = stable_payload_hash(&json!({
            "sessionId":self.session_id,
            "tool":tool_payload(&self.tool),
            "content":self.content,
            "readbackAfterWrite":self.readback_after_write,
            "checkpointId":self.checkpoint_id,
            "checkpointStatus":self.checkpoint_status,
            "approvedChangeFingerprint":self.approved_change_fingerprint
            ,"iterativeCall":self.iterative_call
        }));
        self
    }

    #[must_use]
    pub(crate) fn with_approved_change_fingerprint(
        mut self,
        fingerprint: impl Into<String>,
    ) -> Self {
        self.approved_change_fingerprint = Some(fingerprint.into());
        self.payload_hash = stable_payload_hash(&json!({
            "sessionId":self.session_id,
            "tool":tool_payload(&self.tool),
            "content":self.content,
            "readbackAfterWrite":self.readback_after_write,
            "checkpointId":self.checkpoint_id,
            "checkpointStatus":self.checkpoint_status,
            "approvedChangeFingerprint":self.approved_change_fingerprint
            ,"iterativeCall":self.iterative_call
        }));
        self
    }

    #[must_use]
    pub(crate) fn with_iterative_call(mut self, call: IterativeToolCall) -> Self {
        self.iterative_call = Some(call);
        self.payload_hash = stable_payload_hash(&json!({
            "sessionId":self.session_id,
            "tool":tool_payload(&self.tool),
            "content":self.content,
            "readbackAfterWrite":self.readback_after_write,
            "checkpointId":self.checkpoint_id,
            "checkpointStatus":self.checkpoint_status,
            "approvedChangeFingerprint":self.approved_change_fingerprint,
            "iterativeCall":self.iterative_call
        }));
        self
    }

    #[must_use]
    pub(crate) fn with_optional_checkpoint(mut self, checkpoint_id: Option<&str>) -> Self {
        let Some(checkpoint_id) = checkpoint_id else {
            return self;
        };
        self = self.with_checkpoint(checkpoint_id.to_string(), "ready");
        self
    }

    #[must_use]
    pub(crate) fn with_optional_approved_change_fingerprint(
        mut self,
        fingerprint: Option<&str>,
    ) -> Self {
        let Some(fingerprint) = fingerprint else {
            return self;
        };
        self = self.with_approved_change_fingerprint(fingerprint.to_string());
        self
    }

    #[must_use]
    pub(crate) fn with_optional_iterative_call(mut self, call: Option<&IterativeToolCall>) -> Self {
        let Some(call) = call else {
            return self;
        };
        self = self.with_iterative_call(call.clone());
        self
    }

    #[must_use]
    pub(crate) fn approval_id(&self) -> &str {
        &self.approval_id
    }

    #[must_use]
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub(crate) fn tool(&self) -> &ToolIntent {
        &self.tool
    }

    #[must_use]
    pub(crate) fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    #[must_use]
    pub(crate) fn payload_hash(&self) -> &str {
        &self.payload_hash
    }

    #[must_use]
    pub(crate) fn state(&self) -> PendingAgentActionState {
        self.state
    }

    #[must_use]
    pub(crate) fn readback_after_write(&self) -> bool {
        self.readback_after_write
    }

    #[must_use]
    pub(crate) fn checkpoint_id(&self) -> Option<&str> {
        self.checkpoint_id.as_deref()
    }

    #[must_use]
    pub(crate) fn approved_change_fingerprint(&self) -> Option<&str> {
        self.approved_change_fingerprint.as_deref()
    }

    #[must_use]
    pub(crate) fn iterative_call(&self) -> Option<&IterativeToolCall> {
        self.iterative_call.as_ref()
    }

    #[must_use]
    pub(crate) fn approval_details(&self) -> Option<Value> {
        match &self.tool {
            ToolIntent::GitCommit { message, .. } => {
                let changed_files = self
                    .content
                    .as_deref()
                    .and_then(|content| serde_json::from_str::<Value>(content).ok())?
                    .get("changedFiles")?
                    .clone();
                Some(json!({"message":message,"changedFiles":changed_files}))
            }
            _ => None,
        }
    }

    pub(crate) fn mark_applied(&mut self) {
        self.state = PendingAgentActionState::Applied;
    }

    pub(crate) fn mark_applying(&mut self) {
        self.state = PendingAgentActionState::Applying;
    }

    pub(crate) fn mark_failed(&mut self) {
        self.state = PendingAgentActionState::Failed;
    }

    pub(crate) fn mark_interrupted(&mut self) {
        self.state = PendingAgentActionState::Interrupted;
    }

    #[must_use]
    pub(crate) fn to_json(&self) -> serde_json::Value {
        json!({
            "approvalId":self.approval_id,
            "sessionId":self.session_id,
            "tool":tool_payload(&self.tool),
            "content":self.content,
            "payloadHash":self.payload_hash,
            "state":pending_state_value(self.state),
            "readbackAfterWrite":self.readback_after_write,
            "checkpointId":self.checkpoint_id,
            "checkpointStatus":self.checkpoint_status,
            "approvedChangeFingerprint":self.approved_change_fingerprint
            ,"iterativeCall":self.iterative_call
        })
    }

    pub(crate) fn from_json(value: &serde_json::Value) -> Option<Self> {
        let approval_id = value.get("approvalId")?.as_str()?.to_string();
        let session_id = value.get("sessionId")?.as_str()?.to_string();
        let tool = tool_from_payload(value.get("tool")?)?;
        let content = value
            .get("content")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let payload_hash = value.get("payloadHash")?.as_str()?.to_string();
        let state = pending_state_from_value(
            value
                .get("state")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("pending"),
        );
        let readback_after_write = value
            .get("readbackAfterWrite")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let checkpoint_id = value
            .get("checkpointId")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let checkpoint_status = value
            .get("checkpointStatus")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let approved_change_fingerprint = value
            .get("approvedChangeFingerprint")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let iterative_call = value
            .get("iterativeCall")
            .filter(|value| !value.is_null())
            .and_then(|value| serde_json::from_value(value.clone()).ok());
        Some(Self {
            approval_id,
            session_id,
            tool,
            content,
            payload_hash,
            state,
            readback_after_write,
            checkpoint_id,
            checkpoint_status,
            approved_change_fingerprint,
            iterative_call,
        })
    }
}

#[must_use]
#[cfg(debug_assertions)]
pub(crate) fn pending_content_for_tool(
    tool: &ToolIntent,
    backend_response: &str,
) -> Option<String> {
    let action = structured_desktoplab_action(backend_response);
    let Some(action) = action else {
        return None;
    };
    let kind = action
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let action_path = action.get("path").and_then(serde_json::Value::as_str);
    match (kind, tool, action_path) {
        (
            "create_file" | "replace_file",
            ToolIntent::FilesystemWrite { path },
            Some(action_path),
        ) if action_path == path => action.get("content")?.as_str().map(ToString::to_string),
        ("create_file" | "replace_file", ToolIntent::FilesystemWrite { .. }, None) => {
            action.get("content")?.as_str().map(ToString::to_string)
        }
        ("patch_file", ToolIntent::FilesystemPatch { path }, Some(action_path))
            if action_path == path =>
        {
            patch_payload(&action)
        }
        ("patch_file", ToolIntent::FilesystemPatch { .. }, None) => patch_payload(&action),
        _ => None,
    }
}

#[must_use]
pub(crate) fn pending_content_for_iterative_call(call: &IterativeToolCall) -> Option<String> {
    let arguments = call.arguments().as_object()?;
    match call.name() {
        "desktoplab.write_file" => arguments.get("content")?.as_str().map(ToString::to_string),
        "desktoplab.patch_file" => Some(
            json!({
                "desktoplabPatch":true,
                "expected":arguments.get("expected")?.as_str()?,
                "replacement":arguments.get("replacement")?.as_str()?
            })
            .to_string(),
        ),
        _ => None,
    }
}

#[cfg(debug_assertions)]
pub(crate) fn structured_file_action_tool(backend_response: &str) -> Option<ToolIntent> {
    let action = structured_desktoplab_action(backend_response)?;
    let kind = action
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !matches!(kind, "create_file" | "replace_file" | "patch_file") {
        return None;
    }
    let path = action.get("path")?.as_str()?.trim();
    safe_structured_action_path(path).then(|| match kind {
        "patch_file" => ToolIntent::filesystem_patch(path),
        _ => ToolIntent::filesystem_write(path),
    })
}

#[cfg(debug_assertions)]
pub(crate) fn structured_action_tool(backend_response: &str) -> Option<ToolIntent> {
    if let Some(tool) = structured_file_action_tool(backend_response) {
        return Some(tool);
    }
    let value = structured_backend_response(backend_response)?;
    provider_tool_intent(&value)
}

#[cfg(debug_assertions)]
pub(crate) fn structured_completion_message(backend_response: &str) -> Option<String> {
    let value = structured_backend_response(backend_response)?;
    if provider_tool_name(&value) != Some("desktoplab.complete") {
        return None;
    }
    let message = value.get("arguments")?.get("message")?.as_str()?.trim();
    (!message.is_empty()).then(|| message.to_string())
}

#[cfg(debug_assertions)]
pub(crate) fn structured_clarification_missing_blocked_action(backend_response: &str) -> bool {
    let Some(value) = structured_backend_response(backend_response) else {
        return false;
    };
    if provider_tool_name(&value) != Some("desktoplab.clarify") {
        return false;
    }
    let arguments = value.get("arguments").and_then(Value::as_object);
    arguments
        .and_then(|arguments| canonical_blocked_action(arguments.get("blockedOn")))
        .is_none()
}

#[cfg(debug_assertions)]
pub(crate) fn unrecognized_tool_output_shape(backend_response: &str) -> String {
    let trimmed = backend_response.trim();
    let (envelope, value) = if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        ("json", Some(value))
    } else if let Some(value) =
        fenced_json_body(trimmed).and_then(|body| serde_json::from_str::<Value>(body).ok())
    {
        ("fenced_json", Some(value))
    } else if let Some(value) =
        balanced_json_object(trimmed).and_then(|body| serde_json::from_str::<Value>(body).ok())
    {
        ("mixed_json", Some(value))
    } else {
        ("invalid_json", None)
    };
    let object = value.as_ref().and_then(Value::as_object);
    let arguments = object.and_then(|fields| fields.get("arguments"));
    let mut top_level_keys = object
        .map(|fields| fields.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    top_level_keys.sort();
    let mut argument_keys = arguments
        .and_then(Value::as_object)
        .map(|fields| fields.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    argument_keys.sort();
    json!({
        "envelope":envelope,
        "tool":value.as_ref().and_then(provider_tool_name),
        "topLevelKeys":top_level_keys,
        "argumentsKind":value_kind(arguments),
        "argumentKeys":argument_keys
    })
    .to_string()
}

#[cfg(debug_assertions)]
fn value_kind(value: Option<&Value>) -> &'static str {
    match value {
        None => "missing",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "boolean",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}

#[cfg(debug_assertions)]
pub(crate) fn provider_output_recovery_evidence(backend_response: &str) -> Option<&'static str> {
    let trimmed = backend_response.trim();
    if parse_structured_backend_value(trimmed).is_some() {
        return None;
    }
    if fenced_json_body(trimmed)
        .and_then(parse_structured_backend_value)
        .is_some()
    {
        return Some("provider_output_recovery:fenced_json");
    }
    if concatenated_json_value(trimmed)
        .as_ref()
        .and_then(parse_structured_backend_value_from_value)
        .is_some()
    {
        return Some("provider_output_recovery:concatenated_json");
    }
    if balanced_json_object(trimmed)
        .and_then(parse_structured_backend_value)
        .is_some()
    {
        return Some("provider_output_recovery:mixed_prose_json");
    }
    if trimmed.contains("desktoplabAction") || trimmed.contains("\"tool\"") {
        return Some("provider_output_recovery:invalid_json");
    }
    None
}

#[cfg(debug_assertions)]
fn safe_structured_action_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains("://")
        && !path.contains('@')
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) fn pending_patch_payload(content: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<Value>(content).ok()?;
    if value.get("desktoplabPatch").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    Some((
        value.get("expected")?.as_str()?.to_string(),
        value.get("replacement")?.as_str()?.to_string(),
    ))
}

pub(crate) fn filesystem_mutation_postcondition_is_satisfied(
    tool: &ToolIntent,
    content: Option<&str>,
    current: &str,
) -> bool {
    match tool {
        ToolIntent::FilesystemWrite { .. } => content.is_some_and(|content| current == content),
        ToolIntent::FilesystemPatch { .. } => {
            content
                .and_then(pending_patch_payload)
                .is_some_and(|(expected, replacement)| {
                    patch_postcondition_is_satisfied(&expected, &replacement, current)
                })
        }
        _ => false,
    }
}

pub(crate) fn patch_postcondition_is_satisfied(
    expected: &str,
    replacement: &str,
    current: &str,
) -> bool {
    let current = normalize_line_endings(current);
    let expected = normalize_line_endings(expected);
    let replacement = normalize_line_endings(replacement);
    !expected.is_empty()
        && !current.contains(&expected)
        && (replacement.is_empty() || current.contains(&replacement))
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(debug_assertions)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingMultiFilePatch {
    path: String,
    expected: String,
    replacement: String,
}

#[cfg(debug_assertions)]
impl PendingMultiFilePatch {
    #[must_use]
    pub(crate) fn new(
        path: impl Into<String>,
        expected: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            expected: expected.into(),
            replacement: replacement.into(),
        }
    }

    #[must_use]
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub(crate) fn expected(&self) -> &str {
        &self.expected
    }

    #[must_use]
    pub(crate) fn replacement(&self) -> &str {
        &self.replacement
    }
}

#[cfg(debug_assertions)]
pub(crate) fn pending_multi_file_patch_payload(
    content: &str,
) -> Option<Vec<PendingMultiFilePatch>> {
    let value = serde_json::from_str::<Value>(content).ok()?;
    if value
        .get("desktoplabMultiFilePatch")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return None;
    }
    let files = value
        .get("files")?
        .as_array()?
        .iter()
        .map(|file| {
            Some(PendingMultiFilePatch::new(
                file.get("path")?.as_str()?,
                file.get("expected")?.as_str()?,
                file.get("replacement")?.as_str()?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    (!files.is_empty()).then_some(files)
}

#[cfg(debug_assertions)]
fn patch_payload(action: &Value) -> Option<String> {
    let expected = action
        .get("expected")
        .or_else(|| action.get("patch").and_then(|patch| patch.get("expected")))?
        .as_str()?;
    let replacement = action
        .get("replacement")
        .or_else(|| {
            action
                .get("patch")
                .and_then(|patch| patch.get("replacement"))
        })?
        .as_str()?;
    Some(
        json!({
            "desktoplabPatch":true,
            "expected":expected,
            "replacement":replacement
        })
        .to_string(),
    )
}

#[must_use]
#[cfg(debug_assertions)]
pub(crate) fn has_structured_file_action(backend_response: &str) -> bool {
    structured_desktoplab_action(backend_response).is_some()
}

#[must_use]
#[cfg(debug_assertions)]
pub(crate) fn display_backend_response(backend_response: &str) -> Option<String> {
    if provider_output_recovery_evidence(backend_response)
        == Some("provider_output_recovery:invalid_json")
    {
        return Some("Provider output was structurally invalid; DesktopLab blocked the unsafe action before execution.".to_string());
    }
    match structured_backend_response(backend_response) {
        Some(value) => value
            .get("assistantMessage")
            .and_then(Value::as_str)
            .filter(|message| !message.trim().is_empty())
            .or_else(|| {
                if provider_tool_name(&value) == Some("desktoplab.complete") {
                    value
                        .get("arguments")?
                        .get("message")?
                        .as_str()
                        .filter(|message| !message.trim().is_empty())
                } else {
                    None
                }
            })
            .map(ToString::to_string),
        None => Some(backend_response.to_string()),
    }
}

#[cfg(debug_assertions)]
fn structured_desktoplab_action(backend_response: &str) -> Option<Value> {
    let value = structured_backend_response(backend_response)?;
    value
        .get("desktoplabAction")
        .cloned()
        .or_else(|| provider_file_action(&value))
}

#[cfg(debug_assertions)]
fn structured_backend_response(backend_response: &str) -> Option<Value> {
    let trimmed = backend_response.trim();
    parse_structured_backend_value(trimmed)
        .or_else(|| fenced_json_body(trimmed).and_then(parse_structured_backend_value))
        .or_else(|| {
            concatenated_json_value(trimmed)
                .and_then(|value| parse_structured_backend_value_from_value(&value))
        })
        .or_else(|| balanced_json_object(trimmed).and_then(parse_structured_backend_value))
}

#[cfg(debug_assertions)]
fn parse_structured_backend_value(candidate: &str) -> Option<Value> {
    let value = serde_json::from_str::<Value>(candidate).ok()?;
    parse_structured_backend_value_from_value(&value)
}

#[cfg(debug_assertions)]
fn parse_structured_backend_value_from_value(value: &Value) -> Option<Value> {
    if value.get("desktoplabAction").is_some()
        || provider_tool_intent(&value).is_some()
        || provider_tool_name(value) == Some("desktoplab.complete")
    {
        return Some(value.clone());
    }
    None
}

#[cfg(debug_assertions)]
fn provider_file_action(value: &Value) -> Option<Value> {
    let tool = provider_tool_name(value)?;
    let arguments = value.get("arguments")?.as_object()?;
    let action = match tool {
        "desktoplab.write_file" => json!({
            "kind":"create_file",
            "path":arguments.get("path")?.as_str()?,
            "content":arguments.get("content")?.as_str()?
        }),
        "desktoplab.patch_file" => {
            let expected = arguments
                .get("expected")
                .or_else(|| {
                    arguments
                        .get("patch")
                        .and_then(|patch| patch.get("expected"))
                })?
                .as_str()?;
            let replacement = arguments
                .get("replacement")
                .or_else(|| {
                    arguments
                        .get("patch")
                        .and_then(|patch| patch.get("replacement"))
                })?
                .as_str()?;
            json!({
                "kind":"patch_file",
                "path":arguments.get("path")?.as_str()?,
                "expected":expected,
                "replacement":replacement
            })
        }
        _ => return None,
    };
    Some(action)
}

#[cfg(debug_assertions)]
pub(super) fn provider_tool_intent(value: &Value) -> Option<ToolIntent> {
    let tool = provider_tool_name(value)?;
    let arguments = value.get("arguments")?.as_object()?;
    canonical_tool_intent(tool, arguments)
}

pub(super) fn canonical_tool_intent(
    tool: &str,
    arguments: &serde_json::Map<String, Value>,
) -> Option<ToolIntent> {
    match tool {
        "desktoplab.list_files" => Some(ToolIntent::filesystem_list(
            arguments
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| !path.is_empty())
                .map(ToString::to_string),
        )),
        "desktoplab.read_file" => Some(ToolIntent::filesystem_read(required_provider_argument(
            arguments, "path",
        )?)),
        "desktoplab.search_text" => Some(ToolIntent::search_text(
            required_provider_argument(arguments, "query")?,
            arguments
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| !path.is_empty())
                .map(ToString::to_string),
        )),
        "desktoplab.write_file" => Some(ToolIntent::filesystem_write(required_provider_argument(
            arguments, "path",
        )?)),
        "desktoplab.patch_file" => Some(ToolIntent::filesystem_patch(required_provider_argument(
            arguments, "path",
        )?)),
        "desktoplab.create_directory" => Some(ToolIntent::filesystem_create_directory(
            required_provider_argument(arguments, "path")?,
        )),
        "desktoplab.move_path" => Some(ToolIntent::filesystem_move(
            required_provider_argument(arguments, "source")?,
            required_provider_argument(arguments, "destination")?,
        )),
        "desktoplab.delete_path" => Some(ToolIntent::filesystem_delete(
            required_provider_argument(arguments, "path")?,
            arguments
                .get("recursive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )),
        "desktoplab.run_terminal" => Some(ToolIntent::terminal_scoped(
            arguments
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            required_provider_argument(arguments, "command")?,
            TerminalRiskClass::Medium,
        )),
        "desktoplab.start_process" => Some(ToolIntent::process_start(
            "",
            "",
            arguments
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            required_provider_argument(arguments, "command")?,
        )),
        "desktoplab.poll_process" => Some(ToolIntent::process_poll(required_provider_argument(
            arguments,
            "processId",
        )?)),
        "desktoplab.write_process_stdin" => Some(ToolIntent::process_stdin(
            required_provider_argument(arguments, "processId")?,
        )),
        "desktoplab.kill_process" => Some(ToolIntent::process_kill(required_provider_argument(
            arguments,
            "processId",
        )?)),
        "desktoplab.run_tests" => Some(ToolIntent::test_run(
            required_provider_argument(arguments, "command")?,
            arguments
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("validate agent work"),
        )),
        "desktoplab.git_status" => Some(ToolIntent::git_status()),
        "desktoplab.git_diff" => Some(ToolIntent::git_diff(
            arguments
                .get("path")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        )),
        "desktoplab.commit_changes" => Some(ToolIntent::git_commit_selected(
            required_provider_argument(arguments, "message")?,
            provider_string_array(arguments, "paths")?,
        )),
        "desktoplab.create_checkpoint" => Some(ToolIntent::create_checkpoint(
            required_provider_argument(arguments, "label")?,
        )),
        "desktoplab.clarify" => Some(match canonical_blocked_action(arguments.get("blockedOn")) {
            Some(blocked_action) => ToolIntent::blocking_clarification(
                required_provider_argument(arguments, "question")?,
                blocked_action,
            ),
            None => ToolIntent::clarify(required_provider_argument(arguments, "question")?),
        }),
        "desktoplab.push_changes" => Some(ToolIntent::git_push(
            required_provider_argument(arguments, "remote")?,
            required_provider_argument(arguments, "branch")?,
        )),
        tool if tool.starts_with("mcp.") => Some(ToolIntent::mcp_invoke(
            tool,
            Value::Object(arguments.clone()),
        )),
        _ => None,
    }
}

fn required_provider_argument<'a>(
    arguments: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Option<&'a str> {
    arguments
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn provider_string_array(
    arguments: &serde_json::Map<String, Value>,
    key: &str,
) -> Option<Vec<String>> {
    let Some(value) = arguments.get(key) else {
        return Some(Vec::new());
    };
    let values = value.as_array()?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|path| !path.trim().is_empty())
                .map(ToString::to_string)
        })
        .collect()
}

#[cfg(debug_assertions)]
fn provider_tool_name(value: &Value) -> Option<&str> {
    let raw = value
        .get("tool")
        .and_then(Value::as_str)
        .or_else(|| value.get("name").and_then(Value::as_str))?;
    (raw.starts_with("desktoplab.") || raw.starts_with("mcp.")).then_some(raw)
}

#[cfg(debug_assertions)]
fn fenced_json_body(candidate: &str) -> Option<&str> {
    let body = candidate.strip_prefix("```")?;
    let body = body.trim_start();
    let body = if let Some(rest) = body.strip_prefix("json") {
        rest.trim_start()
    } else {
        body
    };
    let body = body.strip_prefix('\n').unwrap_or(body);
    let end = body.rfind("```").unwrap_or(body.len());
    Some(body[..end].trim())
}

#[cfg(debug_assertions)]
fn balanced_json_object(candidate: &str) -> Option<&str> {
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in candidate.char_indices() {
        if start.is_none() {
            if ch == '{' {
                start = Some(index);
                depth = 1;
            }
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let object_start = start?;
                    let end = index + ch.len_utf8();
                    let object = &candidate[object_start..end];
                    if object.contains("\"desktoplabAction\"")
                        || object.contains("\"tool\"")
                        || (object.contains("\"name\"") && object.contains("\"arguments\""))
                    {
                        return Some(object);
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(debug_assertions)]
fn concatenated_json_value(candidate: &str) -> Option<Value> {
    let mut stream = serde_json::Deserializer::from_str(candidate).into_iter::<Value>();
    let first = stream.next()?.ok()?;
    stream.next()?.ok()?;
    Some(first)
}

fn tool_payload(tool: &ToolIntent) -> serde_json::Value {
    match tool {
        ToolIntent::FilesystemList { path } => json!({
            "kind":"filesystem.list",
            "path":path
        }),
        ToolIntent::FilesystemRead { path } => json!({
            "kind":"filesystem.read",
            "path":path
        }),
        ToolIntent::SearchText { query, path } => json!({
            "kind":"search.text",
            "query":query,
            "path":path
        }),
        ToolIntent::FilesystemWrite { path } => json!({
            "kind":"filesystem.write",
            "path":path
        }),
        ToolIntent::FilesystemPatch { path } => json!({
            "kind":"filesystem.patch",
            "path":path
        }),
        ToolIntent::FilesystemCreateDirectory { path } => json!({
            "kind":"filesystem.create_directory",
            "path":path
        }),
        ToolIntent::FilesystemMove {
            source,
            destination,
        } => json!({
            "kind":"filesystem.move",
            "source":source,
            "destination":destination
        }),
        ToolIntent::FilesystemDelete { path, recursive } => json!({
            "kind":"filesystem.delete",
            "path":path,
            "recursive":recursive
        }),
        ToolIntent::Terminal { command, .. } => json!({
            "kind":"terminal.command",
            "workspaceId":tool.terminal_workspace_id().unwrap_or_default(),
            "workingDirectory":tool.terminal_working_directory().unwrap_or_default(),
            "command":command,
            "riskClass":terminal_risk_value(tool.terminal_risk_class().unwrap_or(TerminalRiskClass::Medium))
        }),
        ToolIntent::ProcessStart {
            workspace_id,
            session_id,
            working_directory,
            command,
        } => json!({
            "kind":"process.start",
            "workspaceId":workspace_id,
            "sessionId":session_id,
            "workingDirectory":working_directory,
            "command":command
        }),
        ToolIntent::ProcessPoll { process_id } => {
            json!({"kind":"process.poll","processId":process_id})
        }
        ToolIntent::ProcessStdin { process_id } => {
            json!({"kind":"process.stdin","processId":process_id})
        }
        ToolIntent::ProcessKill { process_id } => {
            json!({"kind":"process.kill","processId":process_id})
        }
        ToolIntent::TestRun {
            command, reason, ..
        } => json!({
            "kind":"test.run",
            "workspaceId":tool.terminal_workspace_id().unwrap_or_default(),
            "workingDirectory":tool.terminal_working_directory().unwrap_or_default(),
            "command":command,
            "reason":reason
        }),
        ToolIntent::GitCommit { message, paths } => json!({
            "kind":"git.commit",
            "message":message,
            "paths":paths
        }),
        ToolIntent::GitStatus => json!({
            "kind":"git.status"
        }),
        ToolIntent::GitDiff { path } => json!({
            "kind":"git.diff",
            "path":path
        }),
        ToolIntent::GitPush { remote, branch } => json!({
            "kind":"git.push",
            "remote":remote,
            "branch":branch
        }),
        ToolIntent::CreateCheckpoint { label } => json!({
            "kind":"checkpoint.create",
            "label":label
        }),
        ToolIntent::McpInvoke { tool_id, arguments } => json!({
            "kind":"mcp.invoke",
            "toolId":tool_id,
            "arguments":arguments
        }),
        ToolIntent::Clarify {
            question,
            blocked_action,
        } => json!({
            "kind":"clarify",
            "question":question,
            "blockedOn":blocked_action
        }),
        ToolIntent::RuntimeInstall { runtime_id } => json!({
            "kind":"runtime.install",
            "runtimeId":runtime_id
        }),
    }
}

fn tool_from_payload(value: &serde_json::Value) -> Option<ToolIntent> {
    let kind = value.get("kind")?.as_str()?;
    match kind {
        "filesystem.list" => Some(ToolIntent::filesystem_list(
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .filter(|path| !path.is_empty())
                .map(ToString::to_string),
        )),
        "filesystem.read" => Some(ToolIntent::filesystem_read(value.get("path")?.as_str()?)),
        "search.text" => Some(ToolIntent::search_text(
            value.get("query")?.as_str()?,
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .filter(|path| !path.is_empty())
                .map(ToString::to_string),
        )),
        "filesystem.write" => Some(ToolIntent::filesystem_write(value.get("path")?.as_str()?)),
        "filesystem.patch" => Some(ToolIntent::filesystem_patch(value.get("path")?.as_str()?)),
        "filesystem.create_directory" => Some(ToolIntent::filesystem_create_directory(
            value.get("path")?.as_str()?,
        )),
        "filesystem.move" => Some(ToolIntent::filesystem_move(
            value.get("source")?.as_str()?,
            value.get("destination")?.as_str()?,
        )),
        "filesystem.delete" => Some(ToolIntent::filesystem_delete(
            value.get("path")?.as_str()?,
            value
                .get("recursive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )),
        "terminal.command" => Some(ToolIntent::terminal_workspace(
            value
                .get("workspaceId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            value
                .get("workingDirectory")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            value.get("command")?.as_str()?,
            value
                .get("riskClass")
                .and_then(serde_json::Value::as_str)
                .map(terminal_risk_from_value)
                .unwrap_or(TerminalRiskClass::Medium),
        )),
        "process.start" => Some(ToolIntent::process_start(
            value
                .get("workspaceId")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value
                .get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value
                .get("workingDirectory")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value.get("command")?.as_str()?,
        )),
        "process.poll" => Some(ToolIntent::process_poll(value.get("processId")?.as_str()?)),
        "process.stdin" => Some(ToolIntent::process_stdin(value.get("processId")?.as_str()?)),
        "process.kill" => Some(ToolIntent::process_kill(value.get("processId")?.as_str()?)),
        "test.run" => Some(ToolIntent::test_run(
            value.get("command")?.as_str()?,
            value
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("validate agent work"),
        )),
        "git.commit" => Some(ToolIntent::git_commit_selected(
            value.get("message")?.as_str()?,
            value
                .get("paths")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        )),
        "git.status" => Some(ToolIntent::git_status()),
        "git.diff" => Some(ToolIntent::git_diff(
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string),
        )),
        "git.push" => Some(ToolIntent::git_push(
            value.get("remote")?.as_str()?,
            value.get("branch")?.as_str()?,
        )),
        "checkpoint.create" => Some(ToolIntent::create_checkpoint(value.get("label")?.as_str()?)),
        "mcp.invoke" => Some(ToolIntent::mcp_invoke(
            value.get("toolId")?.as_str()?,
            value.get("arguments")?.clone(),
        )),
        "clarify" => Some(match canonical_blocked_action(value.get("blockedOn")) {
            Some(blocked_action) => {
                ToolIntent::blocking_clarification(value.get("question")?.as_str()?, blocked_action)
            }
            None => ToolIntent::clarify(value.get("question")?.as_str()?),
        }),
        "runtime.install" => Some(ToolIntent::runtime_install(
            value.get("runtimeId")?.as_str()?,
        )),
        _ => None,
    }
}

fn canonical_blocked_action(value: Option<&Value>) -> Option<&str> {
    let blocked_action = value?.as_str()?;
    matches!(
        blocked_action,
        "desktoplab.list_files"
            | "desktoplab.read_file"
            | "desktoplab.search_text"
            | "desktoplab.write_file"
            | "desktoplab.patch_file"
            | "desktoplab.create_directory"
            | "desktoplab.move_path"
            | "desktoplab.delete_path"
            | "desktoplab.run_terminal"
            | "desktoplab.start_process"
            | "desktoplab.poll_process"
            | "desktoplab.write_process_stdin"
            | "desktoplab.kill_process"
            | "desktoplab.run_tests"
            | "desktoplab.git_status"
            | "desktoplab.git_diff"
            | "desktoplab.create_checkpoint"
            | "desktoplab.commit_changes"
            | "desktoplab.push_changes"
    )
    .then_some(blocked_action)
}

fn terminal_risk_value(risk: TerminalRiskClass) -> &'static str {
    match risk {
        TerminalRiskClass::Low => "low",
        TerminalRiskClass::Medium => "medium",
        TerminalRiskClass::High => "high",
    }
}

fn terminal_risk_from_value(value: &str) -> TerminalRiskClass {
    match value {
        "low" => TerminalRiskClass::Low,
        "high" => TerminalRiskClass::High,
        _ => TerminalRiskClass::Medium,
    }
}

fn pending_state_value(state: PendingAgentActionState) -> &'static str {
    match state {
        PendingAgentActionState::Pending => "pending",
        PendingAgentActionState::Applying => "applying",
        PendingAgentActionState::Applied => "applied",
        PendingAgentActionState::Failed => "failed",
        PendingAgentActionState::Interrupted => "interrupted",
    }
}

fn pending_state_from_value(value: &str) -> PendingAgentActionState {
    match value {
        "applying" => PendingAgentActionState::Applying,
        "applied" => PendingAgentActionState::Applied,
        "failed" => PendingAgentActionState::Failed,
        "interrupted" => PendingAgentActionState::Interrupted,
        _ => PendingAgentActionState::Pending,
    }
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::IterativeToolCall;
    use desktoplab_tool_gateway::ToolIntent;
    use serde_json::json;

    use super::{
        PendingAgentAction, PendingAgentActionState, canonical_tool_intent,
        pending_content_for_iterative_call, pending_patch_payload,
    };

    #[test]
    fn applying_and_interrupted_states_survive_serialization() {
        let mut action = PendingAgentAction::new(
            "approval.1",
            "session.1",
            ToolIntent::filesystem_write("notes.md"),
            Some("notes".to_string()),
            true,
        );
        action.mark_applying();
        let mut restored = PendingAgentAction::from_json(&action.to_json()).unwrap();
        assert_eq!(restored.state(), PendingAgentActionState::Applying);

        restored.mark_interrupted();
        let restored = PendingAgentAction::from_json(&restored.to_json()).unwrap();
        assert_eq!(restored.state(), PendingAgentActionState::Interrupted);
    }

    #[test]
    fn canonical_call_identity_survives_approval_journal_serialization() {
        let call = IterativeToolCall::new(
            "call-1",
            "desktoplab.patch_file",
            json!({
                "path":"README.md",
                "expected":"old",
                "replacement":"new",
                "replaceAll":true
            }),
        );
        let action = PendingAgentAction::new(
            "approval.1",
            "session.1",
            ToolIntent::filesystem_patch("README.md"),
            None,
            true,
        )
        .with_iterative_call(call.clone());

        let restored = PendingAgentAction::from_json(&action.to_json()).unwrap();

        assert_eq!(restored.iterative_call(), Some(&call));
        assert_eq!(restored.payload_hash(), action.payload_hash());
    }

    #[test]
    fn native_approval_metadata_comes_from_the_canonical_call() {
        let write = IterativeToolCall::new(
            "call-write",
            "desktoplab.write_file",
            json!({"path":"notes.md","content":""}),
        );
        assert_eq!(
            pending_content_for_iterative_call(&write),
            Some(String::new())
        );

        let patch = IterativeToolCall::new(
            "call-patch",
            "desktoplab.patch_file",
            json!({"path":"notes.md","expected":"old","replacement":"new"}),
        );
        let content = pending_content_for_iterative_call(&patch).unwrap();
        assert_eq!(
            pending_patch_payload(&content),
            Some(("old".to_string(), "new".to_string()))
        );
        assert_eq!(
            canonical_tool_intent(
                patch.name(),
                patch.arguments().as_object().expect("canonical object")
            ),
            Some(ToolIntent::filesystem_patch("notes.md"))
        );
    }

    #[test]
    fn canonical_terminal_intent_ignores_provider_workspace_identity() {
        let arguments = json!({
            "workspaceId":"workspace.foreign",
            "cwd":"crates/core",
            "command":"cargo test"
        });

        let intent = canonical_tool_intent(
            "desktoplab.run_terminal",
            arguments.as_object().expect("canonical object"),
        )
        .expect("terminal intent should parse");

        assert_eq!(intent.terminal_workspace_id(), None);
        assert_eq!(intent.terminal_working_directory(), Some("crates/core"));
    }
}
