use serde_json::Value;

pub(in crate::runtime_routes) fn bool_body_field(body: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_bool()
}

pub(in crate::runtime_routes) fn number_body_field(body: &str, field: &str) -> Option<u32> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
}

pub(in crate::runtime_routes) fn string_body_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(ToString::to_string)
}

pub(in crate::runtime_routes) fn segment(path: &str, index: usize) -> String {
    path.split('/')
        .nth(index + 1)
        .unwrap_or_default()
        .to_string()
}
