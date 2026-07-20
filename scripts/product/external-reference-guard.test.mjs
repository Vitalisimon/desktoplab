import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { assessExternalReferencePolicy } from "./external-reference-guard-core.mjs";

const validReference = {
  remote: "https://github.com/openclaw/example.git",
  commit: "a".repeat(40),
  observedAt: "2026-07-15",
  licenseObserved: "MIT",
  decision: "adapt",
  learningScope: "structured agent runtime evidence",
};

test("accepts ignored, untracked, dependency-free references with complete provenance", () => {
  const references = Array.from({ length: 18 }, (_, index) => ({
    ...validReference,
    remote: `https://github.com/openclaw/example-${index}.git`,
  }));
  assert.deepEqual(assessExternalReferencePolicy({
    ignoreSource: ".external-references/\n",
    trackedFiles: ["src/main.rs"],
    manifestSources: [{ path: "Cargo.toml", source: '[package]\nname = "desktoplab"' }],
    ledger: { schemaVersion: 1, references },
  }), []);
});

test("rejects tracked checkouts and OpenClaw build inputs", () => {
  const failures = assessExternalReferencePolicy({
    ignoreSource: "target/\n",
    trackedFiles: [".external-references/openclaw/README.md"],
    manifestSources: [{ path: "package.json", source: '"@openclaw/acpx": "1.0.0"' }],
    ledger: { schemaVersion: 1, references: [] },
  });
  assert.ok(failures.some((failure) => failure.includes("must ignore")));
  assert.ok(failures.some((failure) => failure.includes("must not be tracked")));
  assert.ok(failures.some((failure) => failure.includes("build input")));
});

test("rejects incomplete or ambiguous provenance", () => {
  const failures = assessExternalReferencePolicy({
    ignoreSource: ".external-references/\n",
    trackedFiles: [],
    manifestSources: [],
    ledger: { schemaVersion: 1, references: [{ ...validReference, commit: "short", decision: "copy" }] },
  });
  assert.ok(failures.some((failure) => failure.includes("full lowercase SHA-1")));
  assert.ok(failures.some((failure) => failure.includes("invalid DesktopLab decision")));
});

test("public export may omit the private ledger but still rejects reference build inputs", () => {
  assert.deepEqual(assessExternalReferencePolicy({
    ignoreSource: ".external-references/\n",
    trackedFiles: ["src/main.rs"],
    manifestSources: [],
    ledger: null,
    requireLedger: false,
  }), []);

  const failures = assessExternalReferencePolicy({
    ignoreSource: ".external-references/\n",
    trackedFiles: [],
    manifestSources: [{ path: "package.json", source: '"@openclaw/acpx": "1.0.0"' }],
    ledger: null,
    requireLedger: false,
  });
  assert.ok(failures.some((failure) => failure.includes("build input")));
});

test("external reference guard files stay focused", () => {
  for (const [file, limit] of [
    ["scripts/product/external-reference-guard-core.mjs", 100],
    ["scripts/product/external-reference-guard.mjs", 80],
  ]) {
    const logical = readFileSync(file, "utf8").split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
    assert.ok(logical <= limit, `${file} has ${logical} logical lines, limit ${limit}`);
  }
});
