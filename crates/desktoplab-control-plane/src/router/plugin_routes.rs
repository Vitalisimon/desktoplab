use std::collections::BTreeMap;

use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use serde_json::{Value, json};

use super::helpers::{approval_json, body_field_or, plugin_id_from_trust_path};
use super::{ApiRouteResponse, LocalApiRouter};

pub(crate) const BUNDLED_ACP_PLUGIN_ID: &str = "plugin.acp";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PluginTrustRecord {
    plugin_id: String,
    user_approved: bool,
}

impl PluginTrustRecord {
    pub(crate) fn approved(plugin_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            user_approved: true,
        }
    }

    fn to_json(&self) -> Value {
        json!({"pluginId":self.plugin_id,"userApproved":self.user_approved})
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            plugin_id: value.get("pluginId")?.as_str()?.to_string(),
            user_approved: value.get("userApproved")?.as_bool()?,
        })
    }
}

pub(crate) fn trust_payload(records: &BTreeMap<String, PluginTrustRecord>) -> Value {
    json!({"records":records.values().map(PluginTrustRecord::to_json).collect::<Vec<_>>()})
}

pub(crate) fn load_plugin_trust(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, PluginTrustRecord>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::PluginTrust, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(PluginTrustRecord::from_json)
        .map(|record| (record.plugin_id.clone(), record))
        .collect())
}

impl LocalApiRouter {
    pub(crate) fn plugins_list(&self) -> ApiRouteResponse {
        let user_approved = self
            .plugin_trust
            .get(BUNDLED_ACP_PLUGIN_ID)
            .is_some_and(|record| record.user_approved);
        ApiRouteResponse::ok(plugins_payload(user_approved))
    }

    pub(crate) fn plugin_trust_response(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let plugin_id = plugin_id_from_trust_path(path);
        if plugin_id != BUNDLED_ACP_PLUGIN_ID {
            return ApiRouteResponse::not_found();
        }
        match self.consume_body_approved_record(
            body,
            &super::helpers::body_field_or(body, "sessionId", "session.local"),
            "plugin.trust",
            &plugin_id,
            None,
        ) {
            Ok(true) => {
                self.plugin_trust.insert(
                    plugin_id.to_string(),
                    PluginTrustRecord::approved(plugin_id),
                );
                self.persist_plugin_trust();
                if let Some(error) = self.state_journal_failure() {
                    return ApiRouteResponse::state_journal_failed(error);
                }
                return ApiRouteResponse::ok(json!({
                    "status":"recorded",
                    "approvalId":body_field_or(body, "approvalId", ""),
                    "pluginId":plugin_id,
                    "trust":"user_approved",
                    "executionEligibility":"disabled",
                    "blockedReasons":["runtime_registration_missing","plugin_integrity_missing_signature"]
                }));
            }
            Err(error) => return ApiRouteResponse::state_journal_failed(error),
            Ok(false) => {}
        }
        let approvals_before = self.approvals.list();
        let approval = self
            .approvals
            .request_operation("session.local", "plugin.trust", plugin_id);
        if let Err(error) = self.persist_agent_approval_journal() {
            self.approvals =
                desktoplab_backend_services::ApprovalService::from_records(approvals_before);
            return ApiRouteResponse::state_journal_failed(error);
        }
        ApiRouteResponse::ok(json!({
            "status":"approval_required",
            "reason":"approval_record_required",
            "approval":approval_json(&approval)
        }))
    }
}

fn plugins_payload(user_approved: bool) -> Value {
    json!({
        "plugins":[{
            "pluginId":BUNDLED_ACP_PLUGIN_ID,
            "displayName":"ACP bridge",
            "trust":if user_approved { "user_approved" } else { "unverified" },
            "status":"blocked",
            "capabilities":["agent.external"],
            "descriptorState":"present",
            "coldManifestState":"present",
            "runtimeRegistration":"not_registered",
            "installSource":"bundled_descriptor",
            "integrityStatus":"missing_signature",
            "executionEligibility":"disabled",
            "executionBoundary":{
                "kind":"display-only",
                "reason":"Descriptor metadata is present, but executable plugin runtime is disabled until provenance, signature and sandbox gates exist."
            },
            "provenance":{
                "descriptorState":"present",
                "coldManifestState":"present",
                "runtimeRegistration":"not_registered",
                "installSource":"bundled_descriptor",
                "integrityStatus":"missing_signature",
                "executionEligibility":"disabled",
                "blockedReasons":[
                    "runtime_registration_missing",
                    "plugin_integrity_missing_signature",
                    "workspace_origin_execution_disabled"
                ]
            },
            "blockedReasons":[
                "runtime_registration_missing",
                "plugin_integrity_missing_signature",
                "workspace_origin_execution_disabled"
            ],
            "trustActions":if user_approved { Vec::<Value>::new() } else { vec![json!({
                "id":"trust",
                "label":"Trust plugin",
                "description":"Record user consent after review. Integrity and runtime gates remain enforced."
            })] }
        }]
    })
}
