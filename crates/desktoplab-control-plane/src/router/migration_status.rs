use desktoplab_storage::migration_plan;
use serde_json::{Value, json};

use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn migration_status(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(migration_status_payload())
    }

    pub(crate) fn migration_status_payload(&self) -> Value {
        migration_status_payload()
    }
}

fn migration_status_payload() -> Value {
    let migrations = migration_plan()
        .iter()
        .map(|migration| {
            json!({
                "id":migration.id(),
                "version":migration.version(),
                "checksum":migration.checksum(),
                "description":migration.description(),
                "reversibilityClass":migration.reversibility_class(),
                "operatorStatus":"declared"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "source":"service_backed",
        "kind":"migration_status",
        "schemaVersion":3,
        "state":"ready",
        "migrations":migrations,
        "legacyConfigFindings":[],
        "unsupportedUpgradePaths":[],
        "redacted":true,
        "maxBytes":64000
    })
}
