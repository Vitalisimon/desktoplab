use desktoplab_acp_plugin::{AcpHostPrompt, AcpSessionHost};
use serde_json::{Value, json};

use crate::LocalApiRouter;

impl AcpSessionHost for LocalApiRouter {
    fn create_session(&mut self, cwd: &str) -> Result<String, String> {
        let opened = self
            .route(
                "POST",
                "/v1/workspaces/open",
                &json!({"path":cwd}).to_string(),
            )
            .ok_or_else(|| "workspace route unavailable".to_string())?;
        let workspace = response_json(opened.status(), opened.body())?;
        let workspace_id = workspace
            .get("workspaceId")
            .and_then(Value::as_str)
            .ok_or_else(|| "workspace id missing".to_string())?;
        let backend_id = self.selected_execution_backend_id();
        let session = self.create_bound_agent_session(&workspace_id, &backend_id);
        Ok(session.session_id().to_string())
    }

    fn prompt(&mut self, session_id: &str, prompt: &str) -> Result<AcpHostPrompt, String> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        let workspace_id = self
            .sessions
            .workspace_id_for(session_id)
            .ok_or_else(|| "session workspace missing".to_string())?;
        let body = json!({
            "workspaceId":workspace_id,
            "executionBackendId":session.execution_backend_id(),
            "prompt":prompt
        });
        let response = self
            .route(
                "POST",
                &format!("/v1/sessions/{session_id}/messages"),
                &body.to_string(),
            )
            .ok_or_else(|| "session prompt route unavailable".to_string())?;
        let payload = response_json(response.status(), response.body())?;
        let state = payload
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("failed");
        let message = payload
            .get("transcript")
            .and_then(Value::as_array)
            .and_then(|entries| {
                entries
                    .iter()
                    .rev()
                    .find(|entry| entry.get("role").and_then(Value::as_str) == Some("assistant"))
            })
            .and_then(|entry| entry.get("content"))
            .and_then(Value::as_str)
            .or_else(|| payload.get("summary").and_then(Value::as_str))
            .unwrap_or("DesktopLab completed the turn.")
            .to_string();
        let stop_reason = match state {
            "completed" => "end_turn",
            "cancelled" => "cancelled",
            "blocked" | "failed" => "refusal",
            _ => "end_turn",
        };
        Ok(AcpHostPrompt {
            message,
            stop_reason: stop_reason.to_string(),
        })
    }

    fn cancel(&mut self, session_id: &str) -> Result<(), String> {
        self.sessions
            .get(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        self.sessions
            .cancel(session_id, "ACP client cancelled prompt turn");
        Ok(())
    }
}

fn response_json(status: &str, body: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
    if status == "200 OK" {
        Ok(value)
    } else {
        Err(value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("DesktopLab request failed")
            .to_string())
    }
}
