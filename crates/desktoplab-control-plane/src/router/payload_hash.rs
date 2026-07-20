use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) fn stable_payload_hash(value: &Value) -> String {
    let digest = Sha256::digest(canonical_json(value).as_bytes());
    format!("sha256:{digest:x}")
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(key, value)| (key.as_str(), canonical_json(value)))
                .collect::<BTreeMap<_, _>>();
            let fields = sorted
                .iter()
                .map(|(key, value)| format!("{key}:{value}"))
                .collect::<Vec<_>>();
            format!("{{{}}}", fields.join(","))
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::String(text) => format!("{text:?}"),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => "null".to_string(),
    }
}
