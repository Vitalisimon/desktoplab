use serde_json::Value;

use crate::ToolCallNormalizationError;

pub(crate) fn validate_value(
    value: &Value,
    schema: &Value,
    path: &str,
) -> Result<(), ToolCallNormalizationError> {
    if !type_matches(value, schema.get("type").and_then(Value::as_str)) {
        return Err(ToolCallNormalizationError::InvalidArgumentType(
            display_path(path),
        ));
    }
    let violates_const = schema
        .get("const")
        .is_some_and(|expected| value != expected);
    let violates_enum = schema
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|allowed| !allowed.contains(value));
    if violates_const || violates_enum || violates_string_bounds(value, schema) {
        return Err(invalid(path));
    }
    validate_numeric_bounds(value, schema, path)?;
    if let Some(items) = value.as_array() {
        validate_array(items, schema, path)?;
    }
    if let Some(object) = value.as_object() {
        validate_object(object, schema, path)?;
    }
    Ok(())
}

fn type_matches(value: &Value, expected: Option<&str>) -> bool {
    match expected {
        Some("string") => value.is_string(),
        Some("boolean") => value.is_boolean(),
        Some("number") => value.is_number(),
        Some("integer") => value.as_i64().is_some() || value.as_u64().is_some(),
        Some("array") => value.is_array(),
        Some("object") => value.is_object(),
        Some("null") => value.is_null(),
        Some(_) => false,
        None => true,
    }
}

fn violates_string_bounds(value: &Value, schema: &Value) -> bool {
    let Some(value) = value.as_str() else {
        return false;
    };
    let length = value.chars().count();
    schema
        .get("minLength")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| length < minimum as usize)
        || schema
            .get("maxLength")
            .and_then(Value::as_u64)
            .is_some_and(|maximum| length > maximum as usize)
}

fn validate_numeric_bounds(
    value: &Value,
    schema: &Value,
    path: &str,
) -> Result<(), ToolCallNormalizationError> {
    let Some(number) = value.as_f64() else {
        return Ok(());
    };
    let out_of_bounds = schema
        .get("minimum")
        .and_then(Value::as_f64)
        .is_some_and(|minimum| number < minimum)
        || schema
            .get("maximum")
            .and_then(Value::as_f64)
            .is_some_and(|maximum| number > maximum);
    if out_of_bounds {
        return Err(invalid(path));
    }
    Ok(())
}

fn validate_array(
    items: &[Value],
    schema: &Value,
    path: &str,
) -> Result<(), ToolCallNormalizationError> {
    let invalid_length = schema
        .get("minItems")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| items.len() < minimum as usize)
        || schema
            .get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|maximum| items.len() > maximum as usize);
    let duplicate = schema
        .get("uniqueItems")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && items
            .iter()
            .enumerate()
            .any(|(index, item)| items[..index].contains(item));
    if invalid_length || duplicate {
        return Err(invalid(path));
    }
    if let Some(item_schema) = schema.get("items") {
        for (index, item) in items.iter().enumerate() {
            validate_value(item, item_schema, &format!("{path}[{index}]"))?;
        }
    }
    Ok(())
}

fn validate_object(
    object: &serde_json::Map<String, Value>,
    schema: &Value,
    path: &str,
) -> Result<(), ToolCallNormalizationError> {
    let properties = schema.get("properties").and_then(Value::as_object);
    for name in schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !object.contains_key(name) {
            return Err(ToolCallNormalizationError::MissingArgument(join_path(
                path, name,
            )));
        }
    }
    for (name, value) in object {
        let property_path = join_path(path, name);
        if let Some(property) = properties.and_then(|properties| properties.get(name)) {
            validate_value(value, property, &property_path)?;
        } else if let Some(additional) = schema
            .get("additionalProperties")
            .filter(|additional| additional.is_object())
        {
            validate_value(value, additional, &property_path)?;
        } else if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            return Err(ToolCallNormalizationError::UnexpectedArgument(
                property_path,
            ));
        }
    }
    Ok(())
}

fn invalid(path: &str) -> ToolCallNormalizationError {
    ToolCallNormalizationError::InvalidArgument(display_path(path))
}

fn join_path(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

fn display_path(path: &str) -> String {
    if path.is_empty() {
        "arguments".to_string()
    } else {
        path.to_string()
    }
}
