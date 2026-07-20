use serde_json::{Value, json};

use super::helpers::string_field;
use super::{ApiRouteResponse, LocalApiRouter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceMemoryRecord {
    memory_id: String,
    workspace_id: String,
    kind: String,
    title: String,
    summary: String,
    decisions: Vec<String>,
    source: String,
    created_at: String,
}

impl WorkspaceMemoryRecord {
    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            memory_id: value.get("memoryId")?.as_str()?.to_string(),
            workspace_id: value.get("workspaceId")?.as_str()?.to_string(),
            kind: string_field(value, "kind", "repo_summary"),
            title: string_field(value, "title", "Workspace memory"),
            summary: string_field(value, "summary", ""),
            decisions: value
                .get("decisions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect(),
            source: string_field(value, "source", "desktoplab"),
            created_at: string_field(value, "createdAt", "1970-01-01T00:00:00Z"),
        })
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "memoryId":self.memory_id,
            "workspaceId":self.workspace_id,
            "kind":self.kind,
            "title":self.title,
            "summary":self.summary,
            "decisions":self.decisions,
            "source":self.source,
            "createdAt":self.created_at,
            "redactionStatus":"clean"
        })
    }

    pub(crate) fn context_text(&self) -> String {
        let title = safe_text(&self.title);
        let summary = safe_text(&self.summary);
        let decisions = self
            .decisions
            .iter()
            .map(|decision| safe_text(decision))
            .filter(|decision| !decision.is_empty())
            .collect::<Vec<_>>()
            .join("\n- ");
        if decisions.is_empty() {
            format!("memory_title={title}\nmemory_summary={summary}")
        } else {
            format!(
                "memory_title={title}\nmemory_summary={summary}\nmemory_decisions:\n- {decisions}"
            )
        }
    }

    pub(crate) fn provenance(&self) -> String {
        format!("workspace-memory:{}", self.memory_id)
    }
}

impl LocalApiRouter {
    pub(crate) fn workspace_memory(&self, path: &str) -> ApiRouteResponse {
        let Some(workspace_id) =
            workspace_id_from_memory_path(path).or_else(|| self.workspace_id())
        else {
            return ApiRouteResponse::not_found();
        };
        if self.workspace_record_for_id(&workspace_id).is_none() {
            return ApiRouteResponse::not_found();
        }
        let memories = self
            .workspace_memories
            .get(&workspace_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        ApiRouteResponse::ok(json!({
            "workspaceId":workspace_id,
            "memories":memories.iter().map(WorkspaceMemoryRecord::to_json).collect::<Vec<_>>()
        }))
    }

    pub(crate) fn remember_workspace_memory(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let Some(workspace_id) =
            workspace_id_from_memory_path(path).or_else(|| self.workspace_id())
        else {
            return ApiRouteResponse::not_found();
        };
        if self.workspace_record_for_id(&workspace_id).is_none() {
            return ApiRouteResponse::not_found();
        }
        let value: Value = serde_json::from_str(body).unwrap_or_else(|_| json!({}));
        let next_index = self
            .workspace_memories
            .get(&workspace_id)
            .map(Vec::len)
            .unwrap_or(0)
            + 1;
        let record = WorkspaceMemoryRecord {
            memory_id: format!("{workspace_id}:memory.{next_index}"),
            workspace_id: workspace_id.clone(),
            kind: safe_text(&string_field(&value, "kind", "repo_summary")),
            title: safe_text(&string_field(&value, "title", "Workspace memory")),
            summary: safe_text(&string_field(&value, "summary", "")),
            decisions: safe_decisions(&value),
            source: safe_text(&string_field(&value, "source", "desktoplab")),
            created_at: "1970-01-01T00:00:00Z".to_string(),
        };
        self.workspace_memories
            .entry(workspace_id.clone())
            .or_default()
            .push(record);
        self.persist_workspace_memory(&workspace_id);
        self.workspace_memory(path)
    }

    pub(crate) fn delete_workspace_memory(&mut self, path: &str) -> ApiRouteResponse {
        let memory_id = path
            .trim_start_matches("/v1/workspaces/memory/")
            .trim_end_matches("/delete")
            .trim_matches('/');
        let mut workspace_id = None;
        for (candidate_workspace_id, memories) in &mut self.workspace_memories {
            let before = memories.len();
            memories.retain(|memory| memory.memory_id != memory_id);
            if memories.len() != before {
                workspace_id = Some(candidate_workspace_id.clone());
                break;
            }
        }
        let Some(workspace_id) = workspace_id else {
            return ApiRouteResponse::not_found();
        };
        self.persist_workspace_memory(&workspace_id);
        ApiRouteResponse::ok(json!({
            "workspaceId":workspace_id,
            "deletedMemoryId":memory_id,
            "status":"deleted"
        }))
    }
}

fn workspace_id_from_memory_path(path: &str) -> Option<String> {
    let trimmed = path
        .trim_start_matches("/v1/workspaces/")
        .trim_end_matches("/memory")
        .trim_matches('/');
    if trimmed.is_empty() || trimmed.starts_with("memory/") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn safe_decisions(value: &Value) -> Vec<String> {
    value
        .get("decisions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(safe_text)
        .collect()
}

fn safe_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_secret_fragment)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_secret_fragment(fragment: &str) -> &str {
    let lower = fragment.to_ascii_lowercase();
    if lower.starts_with("sk-")
        || lower.starts_with("ghp_")
        || lower.starts_with("gho_")
        || lower.starts_with("glpat-")
        || lower.contains("api_key=")
        || lower.contains("access_token=")
        || lower.contains("refresh_token=")
        || lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("token=")
    {
        "[REDACTED]"
    } else {
        fragment
    }
}
