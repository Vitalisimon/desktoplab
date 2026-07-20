use desktoplab_runtime::{RuntimeId, RuntimeManager, RuntimeProvenance};
use serde_json::{Value, json};

use super::helpers::{host_runtime_state, host_supports_mlx_lm, runtime_rank};

#[must_use]
pub fn runtimes_response(managed_ollama: bool) -> Value {
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
            json!({
                "runtimeId":runtime_id,
                "displayName":runtime.name(),
                "ownership":runtime_ownership(runtime_id, managed_ollama),
                "status":host_runtime_state(runtime_id, runtime.state()),
                "detectionSource":"host_probe",
                "capabilities":["llm.chat","api.openai-compatible.local"],
                "install":runtime_install_metadata(runtime_id),
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

fn runtime_ownership(runtime_id: &str, managed_ollama: bool) -> &'static str {
    if runtime_id == "runtime.lm-studio" {
        "externally_managed"
    } else if runtime_id == "runtime.ollama" && !managed_ollama {
        "user_owned"
    } else {
        "desktoplab_managed"
    }
}

fn runtime_install_metadata(runtime_id: &str) -> Value {
    if runtime_id == "runtime.lm-studio" {
        return json!({"supported":false,"blockedReason":"Guided setup"});
    }
    if runtime_id == "runtime.mlx-lm" && !host_supports_mlx_lm() {
        return json!({"supported":false,"blockedReason":"Apple Silicon Mac required"});
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
