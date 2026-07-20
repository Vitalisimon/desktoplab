use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFindingSeverity {
    Incompatibility,
    Warning,
    Deprecation,
    ProofGap,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCompatibilityFinding {
    pub severity: PluginFindingSeverity,
    pub code: String,
    pub path: String,
    pub detail: String,
}

impl PluginCompatibilityFinding {
    pub(super) fn new(
        severity: PluginFindingSeverity,
        code: impl Into<String>,
        path: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            path: path.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCompatibilityReport {
    schema_version: u32,
    pub plugin_id: Option<String>,
    pub compatible: bool,
    pub executed_plugin_code: bool,
    pub used_network: bool,
    pub findings: Vec<PluginCompatibilityFinding>,
}

impl PluginCompatibilityReport {
    pub(super) fn new(
        plugin_id: Option<String>,
        findings: Vec<PluginCompatibilityFinding>,
    ) -> Self {
        let compatible = !findings
            .iter()
            .any(|finding| finding.severity == PluginFindingSeverity::Incompatibility);
        Self {
            schema_version: 1,
            plugin_id,
            compatible,
            executed_plugin_code: false,
            used_network: false,
            findings,
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn markdown(&self) -> String {
        let mut output = format!(
            "# Plugin compatibility\n\n- compatible: `{}`\n- executed plugin code: `false`\n- network: `false`\n\n",
            self.compatible
        );
        for finding in &self.findings {
            output.push_str(&format!(
                "- `{:?}` `{}` at `{}`: {}\n",
                finding.severity, finding.code, finding.path, finding.detail
            ));
        }
        output
    }

    pub fn sarif(&self) -> String {
        let results: Vec<_> = self
            .findings
            .iter()
            .map(|finding| {
                json!({
                    "ruleId":finding.code,
                    "level":sarif_level(finding.severity),
                    "message":{"text":finding.detail},
                    "locations":[{"physicalLocation":{"artifactLocation":{"uri":finding.path}}}]
                })
            })
            .collect();
        serde_json::to_string_pretty(&json!({
            "version":"2.1.0",
            "$schema":"https://json.schemastore.org/sarif-2.1.0.json",
            "runs":[{"tool":{"driver":{"name":"DesktopLab plugin inspector"}},"results":results}]
        }))
        .unwrap()
    }

    pub fn junit(&self) -> String {
        let failures = self
            .findings
            .iter()
            .filter(|finding| finding.severity == PluginFindingSeverity::Incompatibility)
            .count();
        let mut output = format!(
            "<testsuite name=\"desktoplab-plugin-inspector\" tests=\"{}\" failures=\"{}\">",
            self.findings.len().max(1),
            failures
        );
        if self.findings.is_empty() {
            output.push_str("<testcase name=\"compatible\"/>");
        }
        for finding in &self.findings {
            output.push_str(&format!("<testcase name=\"{}\">", xml(&finding.code)));
            if finding.severity == PluginFindingSeverity::Incompatibility {
                output.push_str(&format!("<failure message=\"{}\"/>", xml(&finding.detail)));
            }
            output.push_str("</testcase>");
        }
        output.push_str("</testsuite>");
        output
    }
}

fn sarif_level(severity: PluginFindingSeverity) -> &'static str {
    match severity {
        PluginFindingSeverity::Incompatibility => "error",
        PluginFindingSeverity::Warning | PluginFindingSeverity::Deprecation => "warning",
        PluginFindingSeverity::ProofGap => "note",
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
