import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import { basename, join, resolve } from "node:path";

import { executableAvailable, platformAdapter, validateVisualStep } from "./native-platform-adapters.mjs";

export class NativeVisualEvidenceDriver {
  constructor({ platform, evidenceRoot, environment = process.env, execute = systemExecute } = {}) {
    this.adapter = platformAdapter(platform);
    this.root = resolve(evidenceRoot);
    this.environment = environment;
    this.execute = execute;
    this.records = [];
    mkdirSync(join(this.root, "frames"), { recursive: true, mode: 0o700 });
  }

  capabilities() {
    const selected = {};
    const actions = {};
    for (const kind of ["capture", "inspect", "click", "type", "scroll", "hotkey", "menu", "window"]) {
      const alternatives = this.adapter.alternatives?.[kind];
      if (alternatives) {
        const match = alternatives.find((group) => group.every((item) => executableAvailable(item, this.environment)));
        selected[kind] = match?.[0] ?? null;
        actions[kind] = Boolean(match);
      } else {
        const required = this.adapter.requirements[kind] ?? [];
        actions[kind] = required.every((item) => executableAvailable(item, this.environment));
      }
    }
    return { platform: this.adapter.platform, actions, selected, remediation: this.adapter.remediation };
  }

  run(step) {
    validateVisualStep(step);
    const capabilities = this.capabilities();
    if (!capabilities.actions[step.kind]) {
      const error = new Error(`visual_capability_unavailable:${step.kind}`);
      error.remediation = capabilities.remediation;
      throw error;
    }
    const sequence = this.records.length + 1;
    const outputPath = join(this.root, "frames", `${String(sequence).padStart(3, "0")}-${step.kind}.png`);
    const invocation = this.adapter.invocation(step, outputPath, capabilities);
    const startedAt = new Date().toISOString();
    const result = this.execute(invocation, { environment: this.environment });
    const record = {
      sequence,
      platform: this.adapter.platform,
      kind: step.kind,
      target: redact(step.target),
      selector: step.selector ? redact(step.selector) : null,
      coordinates: step.coordinates ?? null,
      startedAt,
      status: result.status === 0 ? "passed" : "failed",
      outcome: redact(result.stderr || result.stdout || (result.status === 0 ? "completed" : "native_action_failed")),
      remediation: result.status === 0 ? null : capabilities.remediation,
    };
    if (step.kind === "inspect" && result.status === 0) record.accessibility = parseAccessibility(result.stdout);
    if (step.kind === "capture" && result.status === 0) record.frame = frameMetadata(outputPath, step.frameReview);
    this.records.push(record);
    this.#writeManifest("incomplete");
    if (result.status !== 0) throw Object.assign(new Error(`native_visual_action_failed:${step.kind}`), { record });
    return structuredClone(record);
  }

  runWithFrames(step, { frameReview = "potentially_sensitive", expectChange = true } = {}) {
    if (["capture", "inspect"].includes(step?.kind)) throw new Error("visual_action_required_for_before_after");
    const before = this.run({ kind: "capture", target: step.target, frameReview });
    const action = this.run(step);
    const after = this.run({ kind: "capture", target: step.target, frameReview });
    const stored = this.records.find((record) => record.sequence === action.sequence);
    Object.assign(stored, {
      beforeFrame: before.frame.path,
      beforeSha256: before.frame.sha256,
      afterFrame: after.frame.path,
      afterSha256: after.frame.sha256,
      expectChange,
    });
    this.#writeManifest("incomplete");
    return structuredClone(stored);
  }

  finalize({ appVersion, appHash, commit, targetId }) {
    const manifest = this.#writeManifest("complete", {
      appVersion,
      appHash,
      commit,
      targetId,
      finishedAt: new Date().toISOString(),
    });
    return structuredClone(manifest);
  }

  #writeManifest(status, identity = {}) {
    const manifest = {
      kind: "desktoplab.native-visual-evidence",
      schemaVersion: 1,
      evidenceKind: "installed_app_operator_driver",
      agentToolAvailable: false,
      status,
      platform: this.adapter.platform,
      ...identity,
      records: this.records,
    };
    writeFileSync(join(this.root, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, { mode: 0o600 });
    return manifest;
  }
}

export function assessVisualEvidence(manifest) {
  const failures = [];
  if (manifest?.kind !== "desktoplab.native-visual-evidence" || manifest?.evidenceKind !== "installed_app_operator_driver") failures.push("invalid_evidence_kind");
  if (manifest?.agentToolAvailable !== false) failures.push("operator_driver_exposed_as_agent_tool");
  if (manifest?.status !== "complete") failures.push("evidence_incomplete");
  const records = manifest?.records ?? [];
  if (records.some((record) => record.status !== "passed")) failures.push("native_action_failed");
  for (const record of records.filter((entry) => entry.frame)) {
    if (record.frame.byteSize < 1024 || record.frame.width < 320 || record.frame.height < 240) failures.push(`blank_or_invalid_frame:${record.sequence}`);
    if (record.frame.contentReview !== "operator_confirmed_clean") failures.push(`sensitive_frame_not_reviewed:${record.sequence}`);
  }
  for (const record of records.filter((entry) => entry.accessibility)) {
    const viewport = record.accessibility.viewport;
    const nodes = record.accessibility.nodes ?? [];
    if (nodes.length === 0) failures.push(`blank_accessibility_tree:${record.sequence}`);
    for (const node of nodes) {
      if (node.bounds && outside(node.bounds, viewport)) failures.push(`clipped_node:${record.sequence}:${node.id}`);
    }
    for (let left = 0; left < nodes.length; left += 1) for (let right = left + 1; right < nodes.length; right += 1) {
      if (nodes[left].exclusive && nodes[right].exclusive && overlaps(nodes[left].bounds, nodes[right].bounds)) failures.push(`overlap:${record.sequence}:${nodes[left].id}:${nodes[right].id}`);
    }
  }
  for (const record of records.filter((entry) => entry.expectChange)) {
    if (!record.beforeSha256 || record.beforeSha256 === record.afterSha256) failures.push(`stale_ui:${record.sequence}`);
  }
  return { kind: "desktoplab.native-visual-assessment", schemaVersion: 1, status: failures.length === 0 ? "pass" : "blocked", failures };
}

function systemExecute(invocation, { environment }) {
  const command = invocation.resolveCommand?.() ?? invocation.command;
  const result = spawnSync(command, invocation.args, { input: invocation.stdin ?? "", encoding: "utf8", env: environment, shell: false, timeout: 20_000, maxBuffer: 1024 * 1024 });
  return { status: result.status ?? 1, stdout: result.stdout ?? "", stderr: result.error?.message ?? result.stderr ?? "" };
}

function frameMetadata(path, frameReview) {
  const bytes = readFileSync(path);
  const dimensions = pngDimensions(bytes);
  return {
    path: `frames/${basename(path)}`,
    sha256: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
    byteSize: statSync(path).size,
    ...dimensions,
    contentReview: frameReview === "operator_confirmed_clean" ? frameReview : "potentially_sensitive",
  };
}

function pngDimensions(bytes) {
  const signature = bytes.subarray(0, 8).toString("hex");
  if (signature !== "89504e470d0a1a0a" || bytes.subarray(12, 16).toString("ascii") !== "IHDR") return { width: 0, height: 0 };
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

function parseAccessibility(value) {
  try {
    const parsed = JSON.parse(String(value).trim());
    if (!Number.isFinite(parsed?.viewport?.width) || !Number.isFinite(parsed?.viewport?.height) || !Array.isArray(parsed?.nodes)) throw new Error();
    return {
      viewport: { width: parsed.viewport.width, height: parsed.viewport.height },
      nodes: parsed.nodes.slice(0, 2048).map((node, index) => ({
        id: redact(node.id ?? `node-${index}`),
        name: redact(node.name ?? ""),
        bounds: node.bounds,
        exclusive: node.exclusive === true,
      })),
    };
  } catch {
    throw new Error("native_accessibility_output_invalid");
  }
}

function redact(value = "") {
  return String(value)
    .replaceAll(resolve(os.homedir()), "[HOME]")
    .replace(/[A-Z]:\\Users\\[^\\\s]+/gi, "[HOME]")
    .replace(/\b(token|password|secret|api[_-]?key)\s*[=:]\s*\S+/gi, "$1=[REDACTED]")
    .slice(0, 4096);
}

function outside(bounds, viewport) {
  return bounds.x < 0 || bounds.y < 0 || bounds.x + bounds.width > viewport.width || bounds.y + bounds.height > viewport.height;
}

function overlaps(a, b) {
  if (!a || !b) return false;
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}
