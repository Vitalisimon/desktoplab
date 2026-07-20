mod report;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use serde::Deserialize;

pub use report::{PluginCompatibilityFinding, PluginCompatibilityReport, PluginFindingSeverity};

const MAX_METADATA_BYTES: u64 = 256 * 1024;
const MAX_ENTRY_BYTES: u64 = 512 * 1024;
const SUPPORTED_CONTRACT: &str = "1";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StaticPluginManifest {
    plugin_id: String,
    contract_version: String,
    entry: String,
    #[serde(default)]
    sdk_imports: Vec<String>,
    #[serde(default)]
    registrations: Vec<String>,
    #[serde(default)]
    hooks: Vec<String>,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(flatten)]
    unknown: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PackageMetadata {
    name: String,
    version: String,
}

pub struct PluginCompatibilityInspector;

impl PluginCompatibilityInspector {
    pub fn inspect(root: &Path) -> PluginCompatibilityReport {
        let mut findings = Vec::new();
        let manifest_path = root.join("desktoplab-plugin.json");
        let manifest = match read_json::<StaticPluginManifest>(&manifest_path, MAX_METADATA_BYTES) {
            Ok(value) => value,
            Err(code) => {
                findings.push(finding(
                    PluginFindingSeverity::Incompatibility,
                    code,
                    "desktoplab-plugin.json",
                ));
                return PluginCompatibilityReport::new(None, findings);
            }
        };
        validate_manifest(root, &manifest, &mut findings);
        validate_package(root, &manifest, &mut findings);
        PluginCompatibilityReport::new(Some(manifest.plugin_id), findings)
    }
}

fn validate_manifest(
    root: &Path,
    manifest: &StaticPluginManifest,
    findings: &mut Vec<PluginCompatibilityFinding>,
) {
    if manifest.plugin_id.is_empty() || manifest.plugin_id.len() > 128 {
        findings.push(finding(
            PluginFindingSeverity::Incompatibility,
            "invalid_plugin_id",
            "desktoplab-plugin.json",
        ));
    }
    if manifest.contract_version != SUPPORTED_CONTRACT {
        findings.push(finding(
            PluginFindingSeverity::Incompatibility,
            "unsupported_contract_version",
            "desktoplab-plugin.json",
        ));
    }
    for field in manifest.unknown.keys() {
        findings.push(PluginCompatibilityFinding::new(
            PluginFindingSeverity::Warning,
            "unknown_manifest_field",
            "desktoplab-plugin.json",
            field,
        ));
    }
    validate_set(
        &manifest.sdk_imports,
        &["@desktoplab/plugin-sdk/v1"],
        "unsupported_sdk_import",
        findings,
    );
    validate_set(
        &manifest.registrations,
        &["runtime", "provider", "tool", "backend"],
        "unsupported_registration",
        findings,
    );
    validate_set(
        &manifest.hooks,
        &["runtime", "provider", "tool", "backend", "shutdown"],
        "unsupported_hook",
        findings,
    );
    validate_set(
        &manifest.permissions,
        &["llm.chat", "tool.filesystem.write"],
        "unknown_permission",
        findings,
    );
    if !safe_relative_path(&manifest.entry) {
        findings.push(finding(
            PluginFindingSeverity::Incompatibility,
            "unsafe_entry_path",
            "desktoplab-plugin.json",
        ));
        return;
    }
    let entry = root.join(&manifest.entry);
    match fs::metadata(&entry) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_ENTRY_BYTES => {}
        Ok(_) => findings.push(finding(
            PluginFindingSeverity::Incompatibility,
            "entry_invalid_or_too_large",
            &manifest.entry,
        )),
        Err(_) => findings.push(finding(
            PluginFindingSeverity::Incompatibility,
            "entry_missing",
            &manifest.entry,
        )),
    }
    if manifest.sdk_imports.is_empty() || manifest.registrations.is_empty() {
        findings.push(finding(
            PluginFindingSeverity::ProofGap,
            "static_contract_evidence_incomplete",
            "desktoplab-plugin.json",
        ));
    }
}

fn validate_package(
    root: &Path,
    manifest: &StaticPluginManifest,
    findings: &mut Vec<PluginCompatibilityFinding>,
) {
    let path = root.join("package.json");
    match read_json::<PackageMetadata>(&path, MAX_METADATA_BYTES) {
        Ok(package) => {
            if package.name != manifest.plugin_id || package.version.is_empty() {
                findings.push(finding(
                    PluginFindingSeverity::Warning,
                    "package_metadata_mismatch",
                    "package.json",
                ));
            }
        }
        Err(code) => findings.push(finding(
            PluginFindingSeverity::ProofGap,
            code,
            "package.json",
        )),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, limit: u64) -> Result<T, &'static str> {
    let metadata = fs::metadata(path).map_err(|_| "metadata_missing")?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("metadata_invalid_or_too_large");
    }
    let bytes = fs::read(path).map_err(|_| "metadata_unreadable")?;
    serde_json::from_slice(&bytes).map_err(|_| "metadata_malformed")
}

fn validate_set(
    values: &[String],
    allowed: &[&str],
    code: &'static str,
    findings: &mut Vec<PluginCompatibilityFinding>,
) {
    let allowed: BTreeSet<_> = allowed.iter().copied().collect();
    for value in values {
        if !allowed.contains(value.as_str()) {
            findings.push(PluginCompatibilityFinding::new(
                PluginFindingSeverity::Incompatibility,
                code,
                "desktoplab-plugin.json",
                value,
            ));
        }
    }
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn finding(
    severity: PluginFindingSeverity,
    code: &'static str,
    path: &str,
) -> PluginCompatibilityFinding {
    PluginCompatibilityFinding::new(severity, code, path, code)
}
