use desktoplab_runtime::{RuntimeId, RuntimeManager, RuntimeProvenance};
use serde_json::{Value, json};

use super::helpers::{host_runtime_state, runtime_rank};

#[must_use]
pub fn runtimes_response(managed_ollama: bool) -> Value {
    let lm_studio = desktoplab_runtime::discover_system_lm_studio();
    let managed_lm_studio = super::lm_studio_managed::status();
    let managed_mlx_lm = super::mlx_lm_managed::status();
    let mut manager = RuntimeManager::new();
    manager.register_runtime(RuntimeId::new("runtime.ollama"), "Ollama");
    manager.register_runtime(RuntimeId::new("runtime.mlx-lm"), "MLX-LM Server");
    manager.register_runtime(RuntimeId::new("runtime.lm-studio"), "LM Studio");
    let mut inventory = manager.inventory();
    inventory.sort_by_key(|runtime| runtime_rank(runtime.id().as_str()));
    let runtimes = inventory
        .into_iter()
        .map(|runtime| {
            let runtime_id = runtime.id().as_str();
            let runtime_capability =
                crate::local_runtime_capability::LocalRuntimeCapability::for_runtime(runtime_id);
            json!({
                "runtimeId":runtime_id,
                "displayName":runtime.name(),
                "ownership":runtime_ownership(runtime_id, managed_ollama, &lm_studio, managed_lm_studio.as_ref(), managed_mlx_lm.as_ref()),
                "status":runtime_status(runtime_id, runtime.state(), &lm_studio, managed_lm_studio.as_ref(), managed_mlx_lm.as_ref()),
                "detectionSource":"host_probe",
                "capabilities":["llm.chat","api.openai-compatible.local"],
                "install":runtime_install_metadata(runtime_capability),
                "runtimeCapability":runtime_capability.to_json(),
                "connection":runtime_connection(runtime_id, &lm_studio, managed_lm_studio.as_ref(), managed_mlx_lm.as_ref()),
                "provenance":runtime_provenance(runtime_id),
                "lifecycle":{
                    "update":runtime_update_lifecycle(runtime_id),
                    "uninstall":runtime_uninstall_lifecycle(runtime_id)
                },
                "repairActions":[]
            })
        })
        .collect::<Vec<_>>();
    json!({"source":"service_backed","runtimes":runtimes})
}

fn runtime_provenance(runtime_id: &str) -> Value {
    let provenance = RuntimeProvenance::for_runtime(runtime_id, None);
    json!({
        "runtimeId":provenance.runtime_id(),
        "version":provenance.version(),
        "installSource":provenance.install_source(),
        "verificationMethod":provenance.verification_method(),
        "integrity":{
            "state":"unavailable",
            "reason":provenance.integrity().reason()
        }
    })
}

fn runtime_ownership(
    runtime_id: &str,
    managed_ollama: bool,
    lm_studio: &desktoplab_runtime::LmStudioExistingDiscovery,
    managed_lm_studio: Option<&desktoplab_runtime::LmStudioManagedStatus>,
    managed_mlx_lm: Option<&desktoplab_runtime::MlxLmManagedStatus>,
) -> &'static str {
    if runtime_id == "runtime.lm-studio" {
        if let Some(managed) = managed_lm_studio {
            return managed.ownership();
        }
        lm_studio.ownership()
    } else if runtime_id == "runtime.mlx-lm" {
        managed_mlx_lm.map_or("none", desktoplab_runtime::MlxLmManagedStatus::ownership)
    } else if runtime_id == "runtime.ollama" && !managed_ollama {
        "user_owned"
    } else if runtime_id == "runtime.ollama" {
        "desktoplab_managed"
    } else {
        "none"
    }
}

fn runtime_status(
    runtime_id: &str,
    fallback: desktoplab_runtime::RuntimeState,
    lm_studio: &desktoplab_runtime::LmStudioExistingDiscovery,
    managed_lm_studio: Option<&desktoplab_runtime::LmStudioManagedStatus>,
    managed_mlx_lm: Option<&desktoplab_runtime::MlxLmManagedStatus>,
) -> &'static str {
    if runtime_id == "runtime.lm-studio" {
        if let Some(managed) = managed_lm_studio {
            return if managed.ready() { "ready" } else { "degraded" };
        }
        return match lm_studio.state() {
            desktoplab_runtime::LmStudioDiscoveryState::Ready => "ready",
            desktoplab_runtime::LmStudioDiscoveryState::CliMissing => "not_installed",
            _ => "degraded",
        };
    }
    if runtime_id == "runtime.mlx-lm" {
        return managed_mlx_lm
            .map(|managed| if managed.ready() { "ready" } else { "degraded" })
            .unwrap_or_else(|| host_runtime_state(runtime_id, fallback));
    }
    host_runtime_state(runtime_id, fallback)
}

fn runtime_connection(
    runtime_id: &str,
    lm_studio: &desktoplab_runtime::LmStudioExistingDiscovery,
    managed_lm_studio: Option<&desktoplab_runtime::LmStudioManagedStatus>,
    managed_mlx_lm: Option<&desktoplab_runtime::MlxLmManagedStatus>,
) -> Value {
    if runtime_id == "runtime.mlx-lm" {
        return managed_mlx_lm.map_or(Value::Null, |managed| {
            json!({
                "state":if managed.ready() {"ready"} else {"degraded"},
                "endpoint":managed.endpoint(),
                "models":managed.models(),
                "modelRevision":managed.model_revision(),
                "pid":managed.pid(),
                "remediation":if managed.ready() {""} else {"Repair the DesktopLab-managed MLX-LM runtime."}
            })
        });
    }
    if runtime_id != "runtime.lm-studio" {
        return Value::Null;
    }
    if let Some(managed) = managed_lm_studio {
        return json!({
            "state":if managed.ready() {"ready"} else {"degraded"},
            "endpoint":managed.endpoint(),
            "models":managed.models(),
            "version":managed.version(),
            "daemonManaged":true,
            "pid":managed.pid(),
            "remediation":if managed.ready() {""} else {"Repair the DesktopLab-managed LM Studio runtime."}
        });
    }
    json!({
        "state":lm_studio.state().as_str(),
        "endpoint":lm_studio.endpoint(),
        "models":lm_studio.models(),
        "version":lm_studio.version(),
        "daemonManaged":lm_studio.daemon_managed(),
        "remediation":lm_studio.remediation()
    })
}

fn runtime_install_metadata(
    capability: crate::local_runtime_capability::LocalRuntimeCapability,
) -> Value {
    if !capability.allows_setup() {
        return json!({"supported":false,"blockedReason":capability.blocked_reason()});
    }
    json!({"supported":true,"diskRequiredGb":2})
}

fn runtime_update_lifecycle(runtime_id: &str) -> Value {
    if runtime_id == "runtime.lm-studio" {
        return json!({"state":"blocked","label":"External app","reason":"Managed outside DesktopLab."});
    }
    json!({"state":"packaging_managed","label":"Installer managed","reason":"Updates are handled by the DesktopLab installer."})
}

fn runtime_uninstall_lifecycle(runtime_id: &str) -> Value {
    if runtime_id == "runtime.lm-studio" {
        return json!({"state":"blocked","label":"External app","reason":"Remove LM Studio from its own app."});
    }
    json!({"state":"packaging_managed","label":"Installer managed","reason":"Runtime removal is handled by the DesktopLab installer."})
}
