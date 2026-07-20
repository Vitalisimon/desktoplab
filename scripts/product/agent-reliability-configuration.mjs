import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { arch, totalmem } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { DatabaseSync } from "node:sqlite";

import { hashArtifact } from "../packaging/artifact-provenance-core.mjs";
import { localModelProvenance } from "./ollama-model-provenance.mjs";

export { localModelProvenance } from "./ollama-model-provenance.mjs";

export function deriveReliabilityConfiguration({ statePath, repoRoot, model = localModelProvenance(statePath), freeze = releaseFreeze(repoRoot) }) {
  const readiness = statePayload(statePath, "backend_readiness", "local");
  if (readiness?.state !== "ready" || readiness?.modelCapabilities?.modelId !== model.modelId) throw new Error("seed state is not ready for the selected local model");
  const runtime = ollamaVersion(commandOutput("ollama", ["--version"]));
  if (freeze.model.runtimeRef !== model.modelId || freeze.model.digest !== model.digest || freeze.model.quantization !== model.quantization) throw new Error("selected local model differs from release freeze");
  if (freeze.runtime.version !== runtime || readiness.modelCapabilities.fingerprint !== freeze.model.capabilityFingerprint) throw new Error("runtime or capability fingerprint differs from release freeze");
  return {
    model: { id: model.modelId, digest: model.digest, quantization: model.quantization },
    runtime: { id: "runtime.ollama", version: bounded(runtime), backendId: "backend.ollama", backendVersion: "constrained_json.v1" },
    toolSchemaDigest: sourceDigest(repoRoot, toolSchemaFiles),
    approvalMode: approvalMode(statePath),
    contextPlan: { digest: sourceDigest(repoRoot, contextPlanFiles), budgetTokens: freeze.adaptivePolicy.contextWindowTokens, compactionPolicy: "bounded-summary-v1" },
    adaptivePolicy: {
      digest: digest(JSON.stringify(freeze.adaptivePolicy)),
      contextWindowTokens: freeze.adaptivePolicy.contextWindowTokens,
      requestTimeoutSeconds: freeze.adaptivePolicy.requestTimeoutSeconds,
      modelMaximumTokens: freeze.adaptivePolicy.modelMaximumTokens,
    },
    plugins: [],
    harnessVersion: "4.0.0",
    hostCapabilities: {
      os: process.platform,
      arch: arch(),
      memoryClassGb: Math.max(1, Math.round(totalmem() / 1024 ** 3)),
      acceleratorClass: process.platform === "darwin" && arch() === "arm64" ? "apple-unified" : "cpu",
      gpuMemoryClassGb: null,
      unifiedMemory: process.platform === "darwin" && arch() === "arm64",
    },
  };
}

function statePayload(path, kind, subjectId) {
  const database = new DatabaseSync(path, { readOnly: true });
  try {
    const row = database.prepare("select payload from productization_state where kind = ? and subject_id = ?").get(kind, subjectId);
    return row?.payload ? JSON.parse(row.payload) : null;
  } finally { database.close(); }
}

function approvalMode(path) {
  const database = new DatabaseSync(path, { readOnly: true });
  try { return database.prepare("select value from settings where key = ?").get("approval.default_mode")?.value ?? "require_approval"; }
  finally { database.close(); }
}

function sourceDigest(root, paths) {
  return digest(paths.map((path) => `${path}\0${hashArtifact(join(root, path)).sha256}`).join("\0"));
}

function commandOutput(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", timeout: 30_000 });
  if (result.status !== 0) throw new Error(`${command} provenance is unavailable`);
  return result.stdout.trim();
}

function ollamaVersion(output) {
  const version = output.match(/(?:client|ollama) version(?: is)?\s+([^\s]+)/i)?.[1];
  if (!version) throw new Error("Ollama client version is unavailable");
  return version;
}

function releaseFreeze(root) { return JSON.parse(readFileSync(join(root, "evaluation/release-candidate-agent-freeze.json"), "utf8")); }
function bounded(value) { return value.slice(0, 160); }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }

const toolSchemaFiles = [
  "crates/desktoplab-control-plane/src/canonical_tool_executor.rs", "crates/desktoplab-control-plane/src/canonical_tool_files.rs",
  "crates/desktoplab-control-plane/src/canonical_tool_git.rs", "crates/desktoplab-control-plane/src/canonical_tool_process.rs",
  "crates/desktoplab-control-plane/src/canonical_tool_search.rs", "crates/desktoplab-control-plane/src/execution_tool_calling.rs",
  "crates/desktoplab-control-plane/src/router/agent_model_tools.rs", "crates/desktoplab-control-plane/src/router/mcp.rs",
];

const contextPlanFiles = [
  "crates/desktoplab-control-plane/src/router/agent_context.rs",
  "crates/desktoplab-control-plane/src/router/agent_compaction.rs",
  "crates/desktoplab-control-plane/src/router/agent_memory.rs",
];
