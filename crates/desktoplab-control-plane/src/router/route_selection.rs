use serde_json::{Value, json};

use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn route_preference(&self, path: &str, body: &str) -> ApiRouteResponse {
        let (local_route_ready, local_blocked_reason) = self.local_agent_readiness();
        let route_selection = route_runtime_and_model(&self.selected_route_id);
        let runtime_id = route_selection
            .as_ref()
            .and_then(|(runtime_id, _)| runtime_id.as_deref())
            .or(self.readiness.runtime_id());
        let model_id = route_selection
            .as_ref()
            .and_then(|(_, model_id)| model_id.as_deref())
            .or(self.readiness.model_id());
        ApiRouteResponse::ok(
            crate::execution_routes::route_response_for_selection_with_readiness(
                &self.selected_route_id,
                path,
                body,
                local_route_ready,
                local_blocked_reason,
                runtime_id,
                model_id,
                self.readiness.model_capabilities(),
                self.codex_bridge_ready(),
            ),
        )
    }

    pub(crate) fn route_options(&self) -> ApiRouteResponse {
        let (local_route_ready, local_blocked_reason) = self.local_agent_readiness();
        let inventory = self.models_inventory();
        let inventory = serde_json::from_str::<Value>(inventory.body())
            .unwrap_or_else(|_| json!({"models":[]}));
        ApiRouteResponse::ok(
            crate::execution_route_options::route_options_response_from_inventory(
                &self.selected_route_id,
                local_route_ready,
                local_blocked_reason,
                self.readiness.runtime_id(),
                self.readiness.model_id(),
                self.codex_bridge_ready(),
                &inventory,
                self.high_end_runtime.as_ref(),
            ),
        )
    }

    pub(crate) fn runtime_inspect(&self) -> ApiRouteResponse {
        let (local_route_ready, local_blocked_reason) = self.local_agent_readiness();
        let route_selection = route_runtime_and_model(&self.selected_route_id);
        let runtime_id = route_selection
            .as_ref()
            .and_then(|(runtime_id, _)| runtime_id.as_deref())
            .or(self.readiness.runtime_id());
        let model_id = route_selection
            .as_ref()
            .and_then(|(_, model_id)| model_id.as_deref())
            .or(self.readiness.model_id());
        ApiRouteResponse::ok(crate::execution_routes::runtime_inspect_response(
            &self.selected_route_id,
            local_route_ready,
            local_blocked_reason,
            runtime_id,
            model_id,
            self.readiness.model_capabilities(),
            self.codex_bridge_ready(),
        ))
    }

    pub(crate) fn update_route_selection(&mut self, body: &str) -> ApiRouteResponse {
        let Some(route_id) = route_id(body) else {
            return ApiRouteResponse::bad_request(
                json!({"code":"INVALID_ROUTE_SELECTION","message":"Choose an execution route."}),
            );
        };

        let options = self.route_options();
        let options =
            serde_json::from_str::<Value>(options.body()).unwrap_or_else(|_| json!({"options":[]}));
        let Some(option) = route_option(&options, &route_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"ROUTE_UNAVAILABLE",
                "message":unavailable_route_message(&route_id)
            }));
        };
        if option["status"].as_str() != Some("available") {
            let reason = option["disabledReason"]
                .as_str()
                .unwrap_or("This execution route is not available.");
            return ApiRouteResponse::bad_request(
                json!({"code":"ROUTE_UNAVAILABLE","message":reason}),
            );
        }

        if option["backendKind"].as_str() == Some("local")
            && let (Some(runtime_id), Some(model_id)) =
                (option["runtimeId"].as_str(), option["modelId"].as_str())
        {
            let previous_readiness = self.readiness.clone();
            let previous_route_id = self.selected_route_id.clone();
            self.readiness.mark_model_verified(
                runtime_id.to_string(),
                model_id.to_string(),
                "selected from verified local model inventory".to_string(),
            );
            if let Some(pull_ref) = crate::model_routes::model_pull_ref(model_id) {
                self.refresh_ollama_model_capabilities(runtime_id, &pull_ref);
            }
            self.selected_route_id = route_id.clone();
            let (agent_ready, blocked_reason) = self.local_agent_readiness();
            if !agent_ready {
                self.readiness = previous_readiness;
                self.selected_route_id = previous_route_id;
                self.persist_readiness_state();
                return ApiRouteResponse::bad_request(json!({
                    "code":"MODEL_AGENT_PROTOCOL_UNAVAILABLE",
                    "message":"DesktopLab could not verify this model's agent protocol.",
                    "reason":blocked_reason.unwrap_or("model_tool_protocol_uncertified")
                }));
            }
        }

        self.selected_route_id = route_id;
        self.stability.mark_route_decision();
        self.persist_readiness_state();
        self.persist_selected_route_id();
        self.route_options()
    }

    pub(crate) fn selected_agent_route_readiness(&self) -> (bool, Option<&'static str>) {
        match self.selected_route_id.as_str() {
            "route.external.codex" if self.codex_bridge_ready() => (true, None),
            "route.external.codex" => (false, Some("external_agent_bridge_unavailable")),
            "route.high-end-local" => (false, Some("model_tool_protocol_uncertified")),
            route if route.starts_with("route.local.") => self.local_agent_readiness(),
            _ => (false, Some("execution_route_unavailable")),
        }
    }

    pub(crate) fn local_agent_readiness(&self) -> (bool, Option<&'static str>) {
        if !self.readiness.is_ready() {
            return (false, self.readiness.blocked_reason());
        }
        let Some(model_id) =
            crate::execution_routes::local_model_id_from_route_id(&self.selected_route_id)
                .or_else(|| self.readiness.model_id().map(ToString::to_string))
        else {
            return (false, Some("model_not_verified"));
        };
        let Some(pull_ref) = crate::model_routes::model_pull_ref(&model_id) else {
            return (false, Some("model_not_in_agent_catalog"));
        };
        if self.readiness.model_id() != Some(model_id.as_str()) {
            return (false, Some("model_not_verified"));
        }
        let Some(capabilities) = self.readiness.model_capabilities() else {
            return (false, Some("model_tool_protocol_uncertified"));
        };
        if capabilities.model_id() != pull_ref {
            return (false, Some("model_tool_protocol_uncertified"));
        }
        if capabilities.capability_state("tools")
            == desktoplab_backends::ModelCapabilityState::Unsupported
        {
            return (false, Some("model_native_tools_unsupported"));
        }
        if !capabilities.tool_protocol_certified() {
            return (false, Some("model_tool_protocol_uncertified"));
        }
        (true, None)
    }

    pub(crate) fn codex_bridge_ready(&self) -> bool {
        let Some(account) = self.provider_accounts.get("provider.openai") else {
            return false;
        };
        let Some(vault_ref) = account.vault_ref() else {
            return false;
        };
        let Some(responder_url) = account.bridge_responder_url() else {
            return false;
        };
        account.is_codex_bridge_ready()
            && self.codex_credential_available(vault_ref)
            && crate::provider_bridge_routes::codex_responder_reachable(responder_url)
    }
}

fn route_id(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("routeId")?
        .as_str()
        .map(str::to_string)
}

fn route_option<'a>(options: &'a Value, route_id: &str) -> Option<&'a Value> {
    options["options"]
        .as_array()?
        .iter()
        .find(|option| option["routeId"].as_str() == Some(route_id))
}

fn unavailable_route_message(route_id: &str) -> &'static str {
    if route_id.starts_with("route.local.") {
        "This model is not ready on this computer."
    } else {
        "This execution route is not available."
    }
}

fn route_runtime_and_model(route_id: &str) -> Option<(Option<String>, Option<String>)> {
    let model_id = crate::execution_routes::local_model_id_from_route_id(route_id)?;
    let runtime_id = desktoplab_model_manager::ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == model_id)
        .map(|variant| variant.runtime_compatibility().runtime_id().to_string());
    Some((runtime_id, Some(model_id)))
}
