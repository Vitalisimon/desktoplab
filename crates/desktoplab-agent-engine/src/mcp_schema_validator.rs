use serde_json::Value;

pub(crate) fn validate_supported_schema(schema: &Value) -> Result<(), String> {
    let object = schema
        .as_object()
        .ok_or_else(|| "mcp_tool_schema_invalid".to_string())?;
    for keyword in object.keys() {
        if !SUPPORTED_SCHEMA_KEYWORDS.contains(&keyword.as_str()) {
            return Err(format!("mcp_tool_schema_keyword_unsupported:{keyword}"));
        }
    }
    validate_schema_shape(object)?;
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for property in properties.values() {
            validate_supported_schema(property)?;
        }
    }
    if let Some(items) = object.get("items") {
        validate_supported_schema(items)?;
    }
    if let Some(additional) = object
        .get("additionalProperties")
        .filter(|value| value.is_object())
    {
        validate_supported_schema(additional)?;
    }
    Ok(())
}

fn validate_schema_shape(object: &serde_json::Map<String, Value>) -> Result<(), String> {
    if object.get("type").is_some_and(|value| {
        !matches!(
            value.as_str(),
            Some("string" | "boolean" | "number" | "integer" | "array" | "object" | "null")
        )
    }) || object
        .get("properties")
        .is_some_and(|value| !value.is_object())
        || object.get("items").is_some_and(|value| !value.is_object())
        || object.get("required").is_some_and(|value| {
            value
                .as_array()
                .is_none_or(|items| items.iter().any(|item| !item.is_string()))
        })
        || object
            .get("additionalProperties")
            .is_some_and(|value| !value.is_boolean() && !value.is_object())
        || object.get("enum").is_some_and(|value| !value.is_array())
    {
        return Err("mcp_tool_schema_invalid".to_string());
    }
    Ok(())
}

const SUPPORTED_SCHEMA_KEYWORDS: &[&str] = &[
    "type",
    "properties",
    "required",
    "additionalProperties",
    "enum",
    "const",
    "minimum",
    "maximum",
    "minItems",
    "maxItems",
    "uniqueItems",
    "items",
    "minLength",
    "maxLength",
    "description",
    "title",
    "default",
    "examples",
];
