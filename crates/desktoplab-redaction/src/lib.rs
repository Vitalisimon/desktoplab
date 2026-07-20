#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedactionResult {
    value: String,
    redacted: bool,
}

impl RedactionResult {
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn redacted(&self) -> bool {
        self.redacted
    }
}

#[must_use]
pub fn redact_sensitive(value: &str) -> String {
    redact_sensitive_with_status(value).value
}

#[must_use]
pub fn redact_sensitive_bounded(value: &str, max_chars: usize) -> RedactionResult {
    let redacted = redact_sensitive_with_status(value);
    let bounded = redacted.value.chars().take(max_chars).collect::<String>();
    let truncated = redacted.value.chars().count() > bounded.chars().count();
    RedactionResult {
        value: bounded,
        redacted: redacted.redacted || truncated,
    }
}

#[must_use]
pub fn redact_sensitive_with_status(value: &str) -> RedactionResult {
    let block_redacted = redact_private_key_blocks(value);
    let token_redacted = redact_token_material(&block_redacted);
    let redacted = token_redacted != value;
    RedactionResult {
        value: token_redacted,
        redacted,
    }
}

#[must_use]
pub fn redact_repository_context(value: &str) -> RedactionResult {
    let mut changed = false;
    let value = value
        .lines()
        .map(|line| {
            let result = redact_sensitive_with_status(line);
            changed |= result.redacted();
            result.value().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    RedactionResult {
        value,
        redacted: changed,
    }
}

fn redact_private_key_blocks(value: &str) -> String {
    let mut in_private_key = false;
    let mut output = String::with_capacity(value.len());
    for line in value.split_inclusive('\n') {
        let logical_line = line
            .strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line);
        let line_ending = &line[logical_line.len()..];
        let upper = logical_line.to_ascii_uppercase();
        if upper.contains("-----BEGIN") && upper.contains("PRIVATE KEY-----") {
            output.push_str("[REDACTED]");
            output.push_str(line_ending);
            in_private_key = true;
            continue;
        }
        if in_private_key {
            if upper.contains("-----END") && upper.contains("PRIVATE KEY-----") {
                in_private_key = false;
            }
            continue;
        }
        output.push_str(line);
    }
    output
}

fn redact_token_material(value: &str) -> String {
    let mut remaining_forced_redactions = 0usize;
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < value.len() {
        let is_whitespace = value[cursor..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace);
        let end = value[cursor..]
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace() != is_whitespace)
            .map_or(value.len(), |(offset, _)| cursor + offset);
        let part = &value[cursor..end];
        cursor = end;
        if is_whitespace {
            output.push_str(part);
            continue;
        }
        if remaining_forced_redactions > 0 {
            output.push_str(&redact_forced_part(part));
            remaining_forced_redactions -= 1;
            if part.eq_ignore_ascii_case("bearer") {
                remaining_forced_redactions = remaining_forced_redactions.max(1);
            }
            continue;
        }
        let (redacted, next_count) = redact_part(part);
        output.push_str(&redacted);
        remaining_forced_redactions = next_count;
    }
    output
}

fn redact_forced_part(part: &str) -> String {
    let end = next_value_boundary(part, 0);
    let suffix = &part[end..];
    let (redacted_suffix, _) = redact_assignments(suffix);
    format!("[REDACTED]{redacted_suffix}")
}

fn redact_part(part: &str) -> (String, usize) {
    let lower = part.to_ascii_lowercase();
    if lower == "bearer" || lower.ends_with(":bearer") {
        return ("[REDACTED]".to_string(), 1);
    }
    if is_standalone_secret(&lower) {
        return ("[REDACTED]".to_string(), 0);
    }
    if is_header_marker(&lower) {
        return (format!("{}[REDACTED]", prefix_before_marker(part)), 2);
    }
    redact_assignments(part)
}

fn redact_assignments(part: &str) -> (String, usize) {
    let mut output = String::new();
    let mut index = 0usize;
    let mut cursor = 0usize;
    let mut force_next = 0usize;
    let mut changed = false;
    let bytes = part.as_bytes();
    while index < part.len() {
        if bytes[index] == b'=' || bytes[index] == b':' {
            let key_start = previous_boundary(part, index);
            let key = normalize_key(&part[key_start..index]);
            if is_sensitive_key(&key) {
                output.push_str(&part[cursor..=index]);
                let value_start = index + 1;
                let value_end = next_value_boundary(part, value_start);
                let raw_value = &part[value_start..value_end];
                if raw_value.contains("[REDACTED") {
                    output.push_str(raw_value);
                } else {
                    output.push_str(redacted_value(raw_value));
                    changed = true;
                }
                if raw_value.to_ascii_lowercase().contains("bearer") {
                    force_next = 1;
                }
                cursor = value_end;
                index = value_end;
                continue;
            }
        }
        index += 1;
    }
    if changed {
        output.push_str(&part[cursor..]);
        (output, force_next)
    } else {
        (part.to_string(), force_next)
    }
}

fn redacted_value(raw: &str) -> &'static str {
    if raw.starts_with('"') && raw.ends_with('"') {
        "\"[REDACTED]\""
    } else if raw.starts_with('"') {
        "\"[REDACTED]"
    } else {
        "[REDACTED]"
    }
}

fn previous_boundary(value: &str, index: usize) -> usize {
    value[..index]
        .rfind(|ch: char| matches!(ch, '&' | '?' | '{' | '[' | ',' | ';' | '\''))
        .map_or(0, |found| found + 1)
}

fn next_value_boundary(value: &str, index: usize) -> usize {
    value[index..]
        .find(|ch: char| matches!(ch, '&' | ';' | ',' | '}' | ']'))
        .map_or(value.len(), |found| index + found)
}

fn normalize_key(key: &str) -> String {
    key.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_lowercase()
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.replace('-', "_");
    [
        "authorization",
        "api_key",
        "apikey",
        "token",
        "secret",
        "cookie",
        "session",
        "provider_key",
        "provider_token",
        "nvapi_key",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn is_header_marker(lower: &str) -> bool {
    lower == "authorization:" || lower == "cookie:" || lower.ends_with(":authorization:")
}

fn prefix_before_marker(part: &str) -> &str {
    part.split_once(':').map_or("", |(prefix, _)| {
        if prefix.is_empty() {
            ""
        } else {
            part.get(..=prefix.len()).unwrap_or("")
        }
    })
}

fn is_standalone_secret(lower: &str) -> bool {
    [
        "sk-", "sk_live_", "sk_test_", "sk-proj-", "ghp_", "glpat-", "xoxb-", "nvapi-", "ai_",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}
mod path_policy;

pub use path_policy::is_secret_bearing_path;
