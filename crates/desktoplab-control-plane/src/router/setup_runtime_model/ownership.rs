use std::path::Path;

use desktoplab_runtime::RuntimeInstallExecutionResult;

pub(super) fn record_desktoplab_runtime_ownership(
    marker_path: Option<&Path>,
    owner_id: Option<&str>,
    runtime_id: &str,
    result: &RuntimeInstallExecutionResult,
) {
    if runtime_id != "runtime.ollama" || !result.desktoplab_started_runtime() {
        return;
    }
    let Some(path) = marker_path else {
        return;
    };
    let Some(owner_id) = owner_id.filter(|owner_id| !owner_id.is_empty()) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{owner_id}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use desktoplab_runtime::{DeterministicProcessRunner, OllamaRuntime, RuntimeInstallExecutor};
    use tempfile::TempDir;

    #[test]
    fn records_ollama_ownership_only_when_executor_started_it_for_this_session() {
        let fixture = TempDir::new().expect("fixture should exist");
        let marker = fixture.path().join("runtime/ollama-owned-by-desktoplab");
        let plan = OllamaRuntime::new()
            .try_platform_install_plan("darwin-arm64")
            .expect("Ollama should support the test platform");
        let existing = RuntimeInstallExecutor::new(DeterministicProcessRunner::succeeds(
            "ollama 0.5.0",
            "",
        ))
        .execute_existing_or_install(&plan);

        record_desktoplab_runtime_ownership(
            Some(&marker),
            Some("desktop-session-1"),
            "runtime.ollama",
            &existing,
        );
        assert!(
            !marker.exists(),
            "unadopted external Ollama must remain untouched"
        );

        let started = RuntimeInstallExecutor::new(DeterministicProcessRunner::sequence(vec![
            (Some(1), "", "command not found"),
            (Some(0), "downloaded", ""),
            (Some(0), "accepted", ""),
            (Some(0), "/Volumes/Ollama", ""),
            (Some(0), "copied", ""),
            (Some(0), "detached", ""),
            (Some(0), "started", ""),
            (Some(0), r#"{"models":[]}"#, ""),
        ]))
        .execute_existing_or_install(&plan);
        record_desktoplab_runtime_ownership(
            Some(&marker),
            Some("desktop-session-1"),
            "runtime.ollama",
            &started,
        );
        let evidence = std::fs::read_to_string(&marker).expect("marker should be written");
        assert_eq!(evidence, "desktop-session-1\n");
    }
}
