#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const patterns = [
  ["private-key", /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/],
  ["aws-access-key", /\bAKIA[0-9A-Z]{16}\b/],
  ["openai-secret", /\bsk-[A-Za-z0-9_-]{20,}\b/],
  ["github-token", /\bgh[ps]_[A-Za-z0-9]{30,}\b/],
];
const allowedFindings = new Set([
  "docs/08-security-trust-model.md:private-key",
  "crates/desktoplab-redaction/tests/redaction_patterns.rs:private-key",
  "crates/desktoplab-tool-gateway/tests/test_runner.rs:openai-secret",
]);
const files = execFileSync("git", ["ls-files", "-z"], { encoding: "utf8" }).split("\0").filter(Boolean);
const findings = [];

for (const file of files) {
  let text;
  try {
    text = readFileSync(file, "utf8");
  } catch {
    continue;
  }
  for (const [kind, pattern] of patterns) {
    if (pattern.test(text) && !allowedFindings.has(`${file}:${kind}`)) {
      findings.push({ file, kind });
    }
  }
}

if (findings.length > 0) {
  console.error(JSON.stringify({ status: "failed", findings }, null, 2));
  process.exit(1);
}
console.log(JSON.stringify({ status: "passed", scannedFiles: files.length, findings: [] }));
