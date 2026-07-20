use serde_json::json;

use super::payload_hash::stable_payload_hash;

#[must_use]
pub(crate) fn git_change_fingerprint(status_entries: &[String], diff_text: &str) -> String {
    stable_payload_hash(&json!({
        "status":status_entries,
        "diff":diff_text,
    }))
}
