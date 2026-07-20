import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  assessEcosystemLearningCertification,
  REQUIRED_SURFACES,
  REQUIRED_TASKS,
} from "./ecosystem-learning-certification-core.mjs";

const planSource = REQUIRED_TASKS.map((task) =>
  `### Task 24.9.${task} - Contract\n\nStatus: implemented\n`,
).join("\n");
const adoptionLedger = {
  schemaVersion: 1,
  tasks: REQUIRED_TASKS.map((task) => ({
    task: `24.9.${task}`,
    outcome: "implemented",
    owner: "desktoplab-core",
    evidence: ["deterministic:test"],
  })),
  personas: REQUIRED_SURFACES.map((surface, index) => ({
    id: `persona-${index}`,
    owner: "release-engineering",
    surfaces: [surface],
    checks: ["deterministic:test"],
  })),
  residualRisks: [{ id: "future", priority: "P2", owner: "product", exitGate: "future plan" }],
  openFindings: [],
};
const referenceLedger = {
  schemaVersion: 1,
  references: Array.from({ length: 18 }, (_, index) => ({
    remote: `https://github.com/openclaw/example-${index}.git`,
    commit: "a".repeat(40),
  })),
};
const evidencePaths = [
  "docs/evidence/openclaw-ecosystem-reference-ledger.json",
  "docs/evidence/openclaw-ecosystem-adoption-ledger.json",
  "docs/evidence/cross-platform-agent-parity.md",
  "docs/evidence/filesystem-race-audit.md",
  "docs/evidence/remote-target-contract.md",
];

function validInput() {
  return {
    planSource,
    adoptionLedger: structuredClone(adoptionLedger),
    referenceLedger,
    trackedFiles: ["crates/desktoplab-storage/src/lib.rs"],
    dependencySources: [{ path: "Cargo.toml", source: '[package]\nname = "desktoplab"' }],
    artifactPaths: ["dist/product/certification.json"],
    evidencePaths,
  };
}

test("accepts complete adoption, six audit surfaces and an isolated build", () => {
  assert.deepEqual(assessEcosystemLearningCertification(validInput()), []);
});

test("rejects missing task truth and an incomplete persona set", () => {
  const input = validInput();
  input.planSource = input.planSource.replace("Status: implemented", "Status: planned");
  input.adoptionLedger.personas.pop();
  const failures = assessEcosystemLearningCertification(input);
  assert.ok(failures.some((failure) => failure.includes("24.9.140")));
  assert.ok(failures.some((failure) => failure.includes("six deterministic")));
});

test("rejects reference checkouts and dependency inheritance", () => {
  const input = validInput();
  input.trackedFiles.push(".external-references/openclaw/package.json");
  input.dependencySources[0].source = '"@openclaw/acpx": "1.0.0"';
  input.artifactPaths.push("dist/node_modules/@openclaw/acpx/index.js");
  const failures = assessEcosystemLearningCertification(input);
  assert.ok(failures.some((failure) => failure.includes("reference checkout")));
  assert.ok(failures.some((failure) => failure.includes("build input")));
  assert.ok(failures.some((failure) => failure.includes("generated artifacts")));
});

test("rejects unowned high-priority findings and residual risks", () => {
  const input = validInput();
  input.adoptionLedger.openFindings = [{ id: "unsafe", priority: "P0", owner: "" }];
  input.adoptionLedger.residualRisks[0].exitGate = "";
  const failures = assessEcosystemLearningCertification(input);
  assert.ok(failures.some((failure) => failure.includes("P0 finding has no owner")));
  assert.ok(failures.some((failure) => failure.includes("owner and exit gate")));
});

test("certification implementation stays bounded", () => {
  const lines = readFileSync(
    "scripts/product/ecosystem-learning-certification-core.mjs",
    "utf8",
  ).split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(lines <= 180, `certification core has ${lines} logical lines, limit 180`);
});
