import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  assessLicenseExpression,
  buildCycloneDx,
  classifyAuditAdvisories,
  classifyLicenses,
  scanText,
} from "./supply-chain-evidence-core.mjs";

test("SPDX policy accepts permissive alternatives and required exceptions", () => {
  assert.equal(assessLicenseExpression("0BSD OR MIT OR Apache-2.0").accepted, true);
  assert.equal(assessLicenseExpression("MIT OR Apache-2.0 OR LGPL-2.1-or-later").accepted, true);
  assert.equal(assessLicenseExpression("Apache-2.0 WITH LLVM-exception OR MIT").accepted, true);
  assert.equal(assessLicenseExpression("MIT/Apache-2.0").accepted, true);
});

test("SPDX policy fails closed for unknown and restricted-only licenses", () => {
  assert.deepEqual(assessLicenseExpression("Mystery-1.0").unknown, ["Mystery-1.0"]);
  assert.equal(assessLicenseExpression("GPL-3.0-only").accepted, false);
  assert.equal(assessLicenseExpression("MIT AND LGPL-2.1-or-later").accepted, false);
  assert.equal(assessLicenseExpression(null).accepted, false);
});

test("license inventory identifies the exact dependency that needs review", () => {
  const report = classifyLicenses([
    { ecosystem: "cargo", name: "good", version: "1.0.0", license: "MIT" },
    { ecosystem: "npm", name: "unknown", version: "2.0.0", license: null },
  ]);
  assert.equal(report.status, "fail");
  assert.equal(report.findings[0].name, "unknown");
});

test("privacy scan catches secrets and host paths without rejecting clean diagnostics", () => {
  assert.deepEqual(scanText({ label: "clean", text: '{"privatePathsIncluded":false}', privateValues: ["/Users/private"] }), []);
  const syntheticToken = ["ghp", "123456789012345678901234567890"].join("_");
  const findings = scanText({ label: "bad", text: `token ${syntheticToken} path /Users/private`, privateValues: ["/Users/private"] });
  assert.deepEqual(findings.map((entry) => entry.kind).sort(), ["github-token", "private-path"]);
});

test("CycloneDX evidence binds source commit and lock hashes", () => {
  const bom = buildCycloneDx({
    commit: "abc123",
    version: "0.1.0",
    cargoMetadata: {
      packages: [{ id: "path+file:///workspace/crate#1.0.0", name: "crate", version: "1.0.0", license: "MIT" }],
      resolve: { nodes: [{ id: "path+file:///workspace/crate#1.0.0", dependencies: [] }] },
    },
    npmPackages: [{ name: "pkg", version: "2.0.0", license: "ISC" }],
    lockHashes: [{ path: "Cargo.lock", sha256: "deadbeef" }],
  });
  assert.equal(bom.bomFormat, "CycloneDX");
  assert.equal(bom.metadata.component["bom-ref"], "desktoplab:abc123");
  assert.equal(bom.metadata.properties[1].value, "deadbeef");
  assert.equal(bom.dependencies[0].ref, "pkg:cargo/crate@1.0.0");
});

test("advisory classification combines workspace Tauri and npm audits", () => {
  const report = classifyAuditAdvisories({
    cargoAudits: [
      { scope: "workspace", report: { vulnerabilities: { list: [] }, warnings: {} } },
      {
        scope: "tauri",
        report: {
          vulnerabilities: { list: [] },
          warnings: { unsound: [{ advisory: { id: "RUSTSEC-test" } }] },
        },
      },
    ],
    npmAudit: { vulnerabilities: { sample: { severity: "high" } } },
  });
  assert.deepEqual(report.findings, [
    { ecosystem: "cargo", scope: "tauri", id: "RUSTSEC-test", severity: "unsound", status: "unclassified" },
    { ecosystem: "npm", scope: "workspace", id: "sample", severity: "high", status: "unclassified" },
  ]);
  assert.equal(report.status, "fail");
});

test("unmaintained transitive crates stay visible without becoming vulnerability claims", () => {
  const report = classifyAuditAdvisories({
    cargoAudits: [{
      scope: "tauri",
      report: {
        vulnerabilities: { list: [] },
        warnings: { unmaintained: [{ advisory: { id: "RUSTSEC-maintenance" } }] },
      },
    }],
    npmAudit: { vulnerabilities: {} },
  });
  assert.equal(report.status, "pass");
  assert.deepEqual(report.notices, [
    { ecosystem: "cargo", scope: "tauri", id: "RUSTSEC-maintenance", severity: "unmaintained", status: "tracked-notice" },
  ]);
});

test("supply-chain scripts stay focused", () => {
  for (const [file, limit] of [["scripts/security/supply-chain-evidence-core.mjs", 300], ["scripts/security/supply-chain-evidence.mjs", 380]]) {
    const logical = readFileSync(file, "utf8").split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
    assert.ok(logical <= limit, `${file} has ${logical} logical lines, limit ${limit}`);
  }
});
