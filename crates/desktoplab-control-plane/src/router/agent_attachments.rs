use desktoplab_redaction::redact_sensitive_with_status;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_EXTERNAL_ATTACHMENTS: usize = 8;
const MAX_ATTACHMENT_CONTENT_BYTES: usize = 64 * 1024;

pub(super) fn external_attachments(body: &str) -> Result<Vec<Value>, Value> {
    let Some(value) = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("externalAttachments").cloned())
    else {
        return Ok(Vec::new());
    };
    let Some(attachments) = value.as_array() else {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENTS_INVALID",
            "External attachments must be an array.",
        ));
    };
    if attachments.len() > MAX_EXTERNAL_ATTACHMENTS {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_LIMIT_EXCEEDED",
            "Attach no more than 8 files at once.",
        ));
    }
    attachments.iter().map(validated_attachment).collect()
}

pub(super) fn external_attachment_text(attachment: &Value) -> Option<(String, String)> {
    let name = attachment.get("name")?.as_str()?.to_string();
    let content = attachment.get("contentText")?.as_str()?;
    let redacted = redact_sensitive_with_status(content).value().to_string();
    Some((name, redacted.chars().take(16_000).collect()))
}

pub(super) fn external_attachment_metadata(attachments: &[Value]) -> Vec<Value> {
    attachments
        .iter()
        .map(|attachment| {
            json!({
                "name":attachment.get("name").and_then(Value::as_str).unwrap_or_default(),
                "size":attachment.get("size").and_then(Value::as_u64).unwrap_or_default(),
                "mediaType":attachment.get("mediaType").and_then(Value::as_str).unwrap_or_default(),
                "contentAttached":attachment.get("contentText").is_some(),
                "contentSha256":attachment.get("contentSha256").and_then(Value::as_str),
                "truncated":attachment.get("truncated").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect()
}

fn validated_attachment(attachment: &Value) -> Result<Value, Value> {
    let name = required_string(attachment, "name").filter(|name| safe_file_name(name));
    let size = attachment.get("size").and_then(Value::as_u64);
    let media_type = required_string(attachment, "mediaType");
    if name.is_none() || size.is_none() || media_type.is_none() {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_INVALID",
            "Each attachment needs a safe name, size and media type.",
        ));
    }
    if !text_like(name.unwrap(), media_type.unwrap()) {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_TYPE_UNSUPPORTED",
            "Only text and source files can be attached to this route.",
        ));
    }
    let Some(content) = attachment.get("contentText").and_then(Value::as_str) else {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_CONTENT_REQUIRED",
            "The selected file could not be read as text.",
        ));
    };
    if content.len() > MAX_ATTACHMENT_CONTENT_BYTES {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_CONTENT_TOO_LARGE",
            "Attachment text exceeds the 64 KiB input limit.",
        ));
    }
    let digest = format!("sha256:{:x}", Sha256::digest(content.as_bytes()));
    if attachment.get("contentSha256").and_then(Value::as_str) != Some(digest.as_str()) {
        return Err(attachment_error(
            "EXTERNAL_ATTACHMENT_DIGEST_MISMATCH",
            "Attachment content does not match its declared digest.",
        ));
    }
    Ok(attachment.clone())
}

fn required_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str().filter(|value| !value.is_empty())
}

fn safe_file_name(name: &str) -> bool {
    name.len() <= 255
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.chars().any(char::is_control)
}

fn text_like(name: &str, media_type: &str) -> bool {
    media_type.starts_with("text/")
        || matches!(
            media_type,
            "application/json" | "application/toml" | "application/xml"
        )
        || name.rsplit_once('.').is_some_and(|(_, extension)| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "txt"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "csv"
                    | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "rs"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
            )
        })
}

fn attachment_error(code: &str, message: &str) -> Value {
    json!({"code":code,"message":message})
}
