use desktoplab_runtime::{
    HighEndRuntimeLifecycle, HttpRuntimeEndpointProbe, RuntimeEndpointHealthProbe,
    RuntimeEndpointSpec, high_end_runtime_contracts,
};
use serde_json::{Value, json};

use crate::{setup_pipeline::SetupPipeline, setup_state::SetupState};

use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn high_end_runtime_discover(&self, body: &str) -> ApiRouteResponse {
        let Ok(payload) = serde_json::from_str::<Value>(body) else {
            return ApiRouteResponse::bad_request(invalid_request("Request body must be JSON."));
        };
        let Some(runtime_id) = payload.get("runtimeId").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(invalid_request("Choose a runtime."));
        };
        let Some(endpoint_url) = payload.get("endpoint").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(invalid_request("Provide a local endpoint."));
        };
        if !high_end_runtime_contracts()
            .iter()
            .any(|contract| contract.runtime_id().as_str() == runtime_id)
        {
            return ApiRouteResponse::bad_request(invalid_request("Unknown high-end runtime."));
        }
        let Ok(endpoint) = RuntimeEndpointSpec::local(endpoint_url, "__model_discovery__") else {
            return ApiRouteResponse::bad_request(json!({
                "code":"INVALID_LOCAL_RUNTIME_ENDPOINT",
                "message":"Use a loopback or private-LAN HTTP endpoint."
            }));
        };
        match HttpRuntimeEndpointProbe::default().discover_models(&endpoint) {
            Ok(models) => ApiRouteResponse::ok(json!({
                "source":"runtime_probe",
                "runtimeId":runtime_id,
                "endpoint":endpoint.base_url(),
                "models":models
            })),
            Err(error) => ApiRouteResponse::bad_request(json!({
                "code":"HIGH_END_RUNTIME_DISCOVERY_FAILED",
                "message":"No compatible local model service answered at this address.",
                "reason":format!("{error:?}")
            })),
        }
    }

    pub(crate) fn high_end_runtime_attach(&mut self, body: &str) -> ApiRouteResponse {
        let Ok(payload) = serde_json::from_str::<Value>(body) else {
            return ApiRouteResponse::bad_request(invalid_request("Request body must be JSON."));
        };
        let Some(runtime_id) = payload.get("runtimeId").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(invalid_request("Choose a runtime."));
        };
        let Some(endpoint_url) = payload.get("endpoint").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(invalid_request("Provide a local endpoint."));
        };
        let Some(model_id) = payload.get("modelId").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(invalid_request("Choose a model."));
        };
        let Some(contract) = high_end_runtime_contracts()
            .into_iter()
            .find(|contract| contract.runtime_id().as_str() == runtime_id)
        else {
            return ApiRouteResponse::bad_request(invalid_request("Unknown high-end runtime."));
        };
        let Ok(endpoint) = RuntimeEndpointSpec::local(endpoint_url, model_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"INVALID_LOCAL_RUNTIME_ENDPOINT",
                "message":"Use an explicit loopback or private-LAN HTTP endpoint with a port."
            }));
        };
        let evidence = HttpRuntimeEndpointProbe::default().probe(&endpoint);
        self.high_end_runtime = Some(HighEndRuntimeLifecycle::attached(
            contract, endpoint, evidence,
        ));
        self.persist_high_end_runtime();
        self.high_end_runtime_inspect()
    }

    pub(crate) fn high_end_runtime_inspect(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(crate::execution_routes::high_end_runtime_health_response(
            self.high_end_runtime.as_ref(),
        ))
    }

    pub(crate) fn high_end_runtime_stop(&mut self) -> ApiRouteResponse {
        let Some(runtime) = self.high_end_runtime.as_mut() else {
            return ApiRouteResponse::bad_request(invalid_request(
                "No high-end runtime is attached.",
            ));
        };
        if runtime.stop_owned().is_err() {
            return ApiRouteResponse::bad_request(json!({
                "code":"USER_OWNED_RUNTIME",
                "message":"DesktopLab will not stop a runtime that you started."
            }));
        }
        self.high_end_runtime_inspect()
    }

    pub(crate) fn reconcile_restored_high_end_route(&mut self) {
        if self.selected_route_id != "route.high-end-local" {
            return;
        }
        let Some(runtime) = self.high_end_runtime.as_ref() else {
            return;
        };
        let runtime_id = runtime.contract().runtime_id().as_str().to_string();
        let model_id = runtime.endpoint().model_id().to_string();
        self.readiness
            .mark_runtime_blocked(&runtime_id, "high-end agent protocol is not certified");
        self.readiness.mark_model_blocked(
            &runtime_id,
            &model_id,
            "model_tool_protocol_uncertified",
        );
        self.setup =
            SetupState::accept(runtime_id.clone(), model_id.clone()).complete(false, false);
        self.setup_pipeline =
            SetupPipeline::select(runtime_id, model_id).block("model_tool_protocol_uncertified");
        self.selected_route_id = crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string();
        self.persist_readiness_state();
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_selected_route_id();
    }
}

fn invalid_request(message: &str) -> Value {
    json!({"code":"INVALID_HIGH_END_RUNTIME_REQUEST","message":message})
}
