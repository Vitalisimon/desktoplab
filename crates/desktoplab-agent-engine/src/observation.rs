use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::IterativeToolCall;

const MAX_OBSERVATION_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservationProvenance {
    evidence_id: String,
    source: String,
    target: Option<String>,
    exit_code: Option<i64>,
    truncated: bool,
}

impl ObservationProvenance {
    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    #[must_use]
    pub fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ToolObservation {
    call_id: String,
    tool_name: String,
    call_signature: String,
    output: Value,
    error: Option<String>,
    provenance: ObservationProvenance,
}

impl ToolObservation {
    #[must_use]
    pub fn success(call: &IterativeToolCall, output: Value) -> Self {
        Self::new(call, output, None)
    }

    pub fn failure(call: &IterativeToolCall, error: impl Into<String>) -> Self {
        Self::new(call, Value::Null, Some(error.into()))
    }

    #[must_use]
    pub fn failure_with_output(
        call: &IterativeToolCall,
        output: Value,
        error: impl Into<String>,
    ) -> Self {
        Self::new(call, output, Some(error.into()))
    }

    fn new(call: &IterativeToolCall, output: Value, error: Option<String>) -> Self {
        let (output, bounded) = bounded_output(output);
        let truncated = bounded || reports_truncation(&output);
        let provenance = ObservationProvenance {
            evidence_id: call.id().to_string(),
            source: call.name().to_string(),
            target: exact_target(call),
            exit_code: output.get("exitCode").and_then(Value::as_i64),
            truncated,
        };
        Self {
            call_id: call.id().to_string(),
            tool_name: call.name().to_string(),
            call_signature: call.signature(),
            output,
            error,
            provenance,
        }
    }

    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    #[must_use]
    pub fn output(&self) -> &Value {
        &self.output
    }

    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[must_use]
    pub fn is_failed_repeat_of(&self, call: &IterativeToolCall) -> bool {
        self.error.is_some() && self.call_signature == call.signature()
    }

    #[must_use]
    pub fn provenance(&self) -> &ObservationProvenance {
        &self.provenance
    }

    pub(crate) fn failure_signature(&self) -> Option<String> {
        self.error
            .as_ref()
            .map(|error| format!("{}:{error}", self.call_signature))
    }

    pub fn is_passing_test_evidence(&self) -> bool {
        matches!(
            self.tool_name.as_str(),
            "desktoplab.run_tests" | "desktoplab.run_terminal"
        ) && self.error.is_none()
            && self.output.get("passed").and_then(Value::as_bool) == Some(true)
            && self.output.get("exitCode").and_then(Value::as_i64) == Some(0)
    }
}

fn exact_target(call: &IterativeToolCall) -> Option<String> {
    call.arguments()
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
}

fn reports_truncation(output: &Value) -> bool {
    ["truncated", "stdoutTruncated", "stderrTruncated"]
        .iter()
        .any(|key| output.get(key).and_then(Value::as_bool) == Some(true))
}

fn bounded_output(output: Value) -> (Value, bool) {
    let serialized = output.to_string();
    if serialized.len() <= MAX_OBSERVATION_BYTES {
        return (output, false);
    }
    let mut bounded = preserved_control_fields(&output);
    bounded.insert("truncated".to_string(), Value::Bool(true));
    bounded.insert("originalBytes".to_string(), json!(serialized.len()));
    let mut preview_limit = MAX_OBSERVATION_BYTES.saturating_sub(4 * 1024);
    loop {
        bounded.insert(
            "preview".to_string(),
            Value::String(bounded_utf8(&serialized, preview_limit).to_string()),
        );
        let value = Value::Object(bounded.clone());
        if value.to_string().len() <= MAX_OBSERVATION_BYTES || preview_limit == 0 {
            return (value, true);
        }
        preview_limit /= 2;
    }
}

fn preserved_control_fields(output: &Value) -> serde_json::Map<String, Value> {
    const FIELDS: &[&str] = &[
        "status",
        "state",
        "exitCode",
        "passed",
        "changed",
        "path",
        "processId",
        "checkpointId",
        "ref",
        "command",
        "mode",
        "caseSensitive",
        "startLine",
        "endLine",
        "totalLines",
        "totalEntries",
        "stdoutTruncated",
        "stderrTruncated",
        "outputTruncated",
    ];
    let mut preserved = serde_json::Map::new();
    for field in FIELDS {
        let Some(value) = output.get(field) else {
            continue;
        };
        if !value.is_string() || value.as_str().is_some_and(|value| value.len() <= 1_024) {
            preserved.insert((*field).to_string(), value.clone());
        }
    }
    preserved
}

fn bounded_utf8(value: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}
