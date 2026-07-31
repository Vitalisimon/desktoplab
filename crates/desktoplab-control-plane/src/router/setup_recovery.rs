use desktoplab_backend_services::JobState;
use desktoplab_runtime::{ProcessCommand, ProcessRunner, SystemProcessRunner};
use serde_json::Value;

use crate::{
    BackendReadinessState,
    setup_pipeline::{SetupPipeline, SetupPipelineState},
    setup_state::SetupState,
};

use super::LocalApiRouter;

impl LocalApiRouter {
    pub(crate) fn reconcile_agent_model_catalog(&mut self) {
        let stale_setup_model = self
            .selected_setup_ids()
            .is_some_and(|(_, model_id)| crate::model_routes::model_pull_ref(&model_id).is_none());
        let stale_readiness_model = self
            .readiness
            .model_id()
            .is_some_and(|model_id| crate::model_routes::model_pull_ref(model_id).is_none());
        let stale_route = match self.selected_route_id.as_str() {
            "route.external.codex" => false,
            "route.high-end-local" => true,
            crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID => false,
            route if route.starts_with("route.local.") => {
                crate::execution_routes::local_model_id_from_route_id(route)
                    .is_none_or(|model_id| crate::model_routes::model_pull_ref(&model_id).is_none())
            }
            _ => true,
        };
        if !stale_setup_model && !stale_readiness_model && !stale_route {
            return;
        }

        self.setup = SetupState::default();
        self.setup_pipeline = SetupPipeline::default();
        self.readiness = BackendReadinessState::default();
        if self.selected_route_id != "route.external.codex" {
            self.selected_route_id =
                crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string();
        }
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_readiness_state();
        self.persist_selected_route_id();
    }

    pub(crate) fn recover_stale_runtime_install(&mut self) {
        if self.setup_pipeline.state() != SetupPipelineState::RuntimeInstalling {
            return;
        }
        let has_terminal_runtime_job = self.jobs.list_jobs().iter().any(|job| {
            job.kind() == "runtime.install"
                && matches!(
                    job.state(),
                    JobState::Failed | JobState::Blocked | JobState::Cancelled
                )
        });
        if !has_terminal_runtime_job {
            return;
        }
        let runtime_id = self
            .setup_pipeline
            .to_json()
            .get("runtimeId")
            .and_then(Value::as_str)
            .unwrap_or("runtime.ollama")
            .to_string();
        let reason = "runtime_install_failed";
        self.setup_pipeline = self.setup_pipeline.clone().block(reason);
        self.setup = self.setup.clone().complete(false, false);
        self.readiness.mark_runtime_blocked(runtime_id, reason);
        for job in self.jobs.list_jobs() {
            if job.kind() == "runtime.install" && job.state() == JobState::Running {
                let _ = self.jobs.block_with_message(job.id(), reason);
            }
        }
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_readiness_state();
        self.persist_runtime_jobs();
    }

    pub(crate) fn recover_existing_host_setup(&mut self) {
        if self.readiness.is_ready() {
            return;
        }
        let Some((runtime_id, model_id)) = self.selected_setup_ids() else {
            return;
        };
        if runtime_id != "runtime.ollama" {
            return;
        }
        let runtime = <SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ProcessCommand::new("ollama").arg("--version"),
        );
        if !runtime.succeeded() {
            return;
        }
        let Some(pull_ref) = crate::model_routes::model_pull_ref(&model_id) else {
            return;
        };
        let desktoplab_managed = self.owns_managed_ollama_runtime();
        let models = <SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ollama_inventory_probe(desktoplab_managed),
        );
        let model_installed = models.succeeded()
            && ollama_inventory_contains_model(desktoplab_managed, &pull_ref, models.stdout());
        self.reconcile_existing_host_setup(
            runtime_id,
            model_id,
            true,
            model_installed,
            "existing runtime detected during startup",
            "existing model detected during startup",
        );
        if model_installed {
            self.refresh_ollama_model_capabilities("runtime.ollama", &pull_ref);
            self.persist_readiness_state();
        }
    }

    pub fn reconcile_existing_host_setup_for_test(
        &mut self,
        runtime_installed: bool,
        model_installed: bool,
    ) {
        let Some((runtime_id, model_id)) = self.selected_setup_ids() else {
            return;
        };
        self.reconcile_existing_host_setup(
            runtime_id,
            model_id,
            runtime_installed,
            model_installed,
            "existing runtime detected during test",
            "existing model detected during test",
        );
    }

    fn reconcile_existing_host_setup(
        &mut self,
        runtime_id: String,
        model_id: String,
        runtime_installed: bool,
        model_installed: bool,
        runtime_evidence: &str,
        model_evidence: &str,
    ) {
        if !runtime_installed {
            return;
        }
        self.readiness
            .mark_runtime_verified(runtime_id.clone(), runtime_evidence.to_string());
        if model_installed {
            self.readiness
                .mark_model_verified(runtime_id, model_id, model_evidence.to_string());
            self.setup_pipeline = self.setup_pipeline.clone().ready();
        } else {
            self.setup_pipeline = SetupPipeline::select(runtime_id, model_id);
        }
        self.setup = self.setup.clone().complete(
            self.readiness.runtime_verified(),
            self.readiness.model_verified(),
        );
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_readiness_state();
    }

    fn selected_setup_ids(&self) -> Option<(String, String)> {
        selected_ids_from(&self.setup.to_json())
            .or_else(|| selected_ids_from(&self.setup_pipeline.to_json()))
    }
}

fn selected_ids_from(value: &Value) -> Option<(String, String)> {
    let runtime_id = value.get("runtimeId")?.as_str()?.to_string();
    let model_id = value.get("modelId")?.as_str()?.to_string();
    if runtime_id.is_empty() || model_id.is_empty() {
        return None;
    }
    Some((runtime_id, model_id))
}

fn ollama_inventory_probe(desktoplab_managed: bool) -> ProcessCommand {
    if desktoplab_managed {
        ProcessCommand::new("ollama").arg("list")
    } else {
        ProcessCommand::new("curl")
            .arg("--fail")
            .arg("http://127.0.0.1:11434/api/tags")
    }
}

fn ollama_inventory_contains_model(
    desktoplab_managed: bool,
    pull_ref: &str,
    inventory_output: &str,
) -> bool {
    if desktoplab_managed {
        return crate::model_routes::verify_model_inventory(pull_ref, inventory_output)
            == desktoplab_model_manager::ModelVerification::passed();
    }

    serde_json::from_str::<Value>(inventory_output)
        .ok()
        .and_then(|inventory| inventory.get("models").and_then(Value::as_array).cloned())
        .is_some_and(|models| {
            models.iter().any(|model| {
                ["name", "model"].iter().any(|field| {
                    model
                        .get(field)
                        .and_then(Value::as_str)
                        .is_some_and(|name| name == pull_ref)
                })
            })
        })
}

#[cfg(test)]
mod tests {
    use super::{ollama_inventory_contains_model, ollama_inventory_probe};

    #[test]
    fn user_owned_recovery_probe_cannot_autostart_ollama() {
        let command = ollama_inventory_probe(false);

        assert_eq!(command.program(), "curl");
        assert_eq!(
            command.args(),
            ["--fail", "http://127.0.0.1:11434/api/tags"]
        );
    }

    #[test]
    fn managed_recovery_may_launch_through_the_owned_cli_inventory() {
        let command = ollama_inventory_probe(true);

        assert_eq!(command.program(), "ollama");
        assert_eq!(command.args(), ["list"]);
    }

    #[test]
    fn user_owned_api_inventory_verifies_exact_model_without_cli_output() {
        let inventory =
            r#"{"models":[{"name":"nemotron-3-nano:4b","model":"nemotron-3-nano:4b"}]}"#;

        assert!(ollama_inventory_contains_model(
            false,
            "nemotron-3-nano:4b",
            inventory
        ));
        assert!(!ollama_inventory_contains_model(
            false, "qwen3:8b", inventory
        ));
    }

    #[test]
    fn malformed_user_owned_inventory_fails_closed() {
        assert!(!ollama_inventory_contains_model(
            false,
            "nemotron-3-nano:4b",
            "not-json"
        ));
    }
}
