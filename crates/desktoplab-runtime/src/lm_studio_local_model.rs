use std::path::{Component, Path};

use serde_json::Value;

use crate::{LmStudioManagedPlan, ProcessCommand, ProcessRunner};

struct TrustedModel {
    catalog_ref: &'static str,
    local_key: &'static str,
    sha256: &'static str,
}

// This is artifact provenance, not a prompt-specific model selection rule.
// The digest is published by lmstudio-community for the MXFP4 GGUF artifact.
const TRUSTED_MODELS: &[TrustedModel] = &[TrustedModel {
    catalog_ref: "openai/gpt-oss-20b",
    local_key: "gpt-oss-20b",
    sha256: "65d06d31a3977d553cb3af137b5c26b5f1e9297a6aaa29ae7caa98788cde53ab",
}];

pub(crate) fn verified_local_model(
    runner: &impl ProcessRunner,
    plan: &LmStudioManagedPlan,
) -> Result<Option<String>, &'static str> {
    let Some(trusted) = TRUSTED_MODELS
        .iter()
        .find(|entry| entry.catalog_ref == plan.model_id())
    else {
        return Ok(None);
    };
    let inventory = runner.run(super::lm_studio_managed_execution::managed(
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("ls")
            .arg("--json"),
        plan,
    ));
    if !inventory.succeeded() {
        return Err("local_model_inventory_failed");
    }
    let models: Vec<Value> = serde_json::from_str(inventory.stdout_for_checks())
        .map_err(|_| "local_model_inventory_invalid")?;
    let matching: Vec<_> = models
        .iter()
        .filter(|model| model["type"] == "llm" && model["modelKey"] == trusted.local_key)
        .collect();
    if matching.is_empty() {
        return Ok(None);
    }
    if matching.len() != 1 || matching[0]["format"] != "gguf" {
        return Err("local_model_ambiguous");
    }
    let relative = matching[0]["path"]
        .as_str()
        .ok_or("local_model_path_invalid")?;
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("local_model_path_invalid");
    }
    let root = plan.managed_home().join(".lmstudio/models");
    let root = root
        .canonicalize()
        .map_err(|_| "local_model_path_invalid")?;
    let file = root
        .join(relative)
        .canonicalize()
        .map_err(|_| "local_model_path_invalid")?;
    if !file.starts_with(&root) || !file.is_file() {
        return Err("local_model_path_invalid");
    }
    let digest = runner.run(
        ProcessCommand::new("shasum")
            .arg("-a")
            .arg("256")
            .arg(file.to_string_lossy()),
    );
    if !digest.succeeded() || digest.stdout().split_whitespace().next() != Some(trusted.sha256) {
        return Err("local_model_integrity_failed");
    }
    Ok(Some(trusted.local_key.to_string()))
}
