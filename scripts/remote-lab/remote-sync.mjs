import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, statSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import { basename, join, resolve } from "node:path";

export function createSourceSnapshot(repoRoot) {
  const commit = git(repoRoot, ["rev-parse", "HEAD"], "utf8").stdout.trim();
  const dirty = git(repoRoot, ["status", "--porcelain", "--untracked-files=all"], "utf8").stdout.trim();
  if (dirty) throw new Error("remote sync requires a clean tracked source tree");
  const archive = git(repoRoot, ["archive", "--format=tar", "HEAD"], null).stdout;
  return {
    kind: "desktoplab.source-snapshot",
    schemaVersion: 1,
    commit,
    treeState: "clean",
    archive,
    archiveSha256: sha256(archive),
    archiveBytes: archive.length,
  };
}

export function syncSourceSnapshot(snapshot, target, runId, transport) {
  if (!/^[a-z0-9][a-z0-9-]{1,63}$/.test(runId)) throw new Error("invalid remote run id");
  const temporary = mkdtempSync(join(os.tmpdir(), "desktoplab-sync-"));
  const localArchive = join(temporary, `${runId}.tar`);
  const remoteArchive = `.desktoplab-remote/${runId}.tar`;
  try {
    writeFileSync(localArchive, snapshot.archive);
    const base = transport.run(target, target.platform === "windows"
      ? "New-Item -ItemType Directory -Force -Path '.desktoplab-remote' | Out-Null"
      : "mkdir -p '.desktoplab-remote'");
    if (base.status !== 0) return incomplete(snapshot, target, "prepare_base_failed", base.reason);
    const uploaded = transport.upload(target, localArchive, remoteArchive);
    if (uploaded.status !== 0) return incomplete(snapshot, target, "upload_failed", uploaded.reason);
    const prepared = transport.run(target, prepareCommand(target.platform, runId, snapshot.archiveSha256));
    if (prepared.status !== 0) return incomplete(snapshot, target, "prepare_failed", prepared.reason);
    if (!prepared.stdout.includes(`DESKTOPLAB_ARCHIVE_SHA256=${snapshot.archiveSha256}`)) {
      return incomplete(snapshot, target, "source_hash_mismatch", null);
    }
    return {
      kind: "desktoplab.remote-sync",
      schemaVersion: 1,
      status: "complete",
      target: targetFingerprint(target),
      source: sourceIdentity(snapshot),
      remoteWorkspace: `.desktoplab-remote/${runId}/source`,
    };
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

export class EvidenceBundle {
  static begin(root, identity) {
    if (existsSync(root)) throw new Error("evidence bundle already exists");
    mkdirSync(join(root, "artifacts"), { recursive: true });
    const manifest = {
      kind: "desktoplab.remote-evidence-bundle",
      schemaVersion: 1,
      status: "incomplete",
      ...identity,
      artifacts: [],
      lifecycle: [{ event: "collection_started", at: identity.startedAt }],
    };
    writeManifest(root, manifest);
    return new EvidenceBundle(root, manifest);
  }

  static resume(root) {
    const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8"));
    if (manifest.status !== "incomplete") throw new Error("only incomplete evidence can resume");
    return new EvidenceBundle(root, manifest);
  }

  constructor(root, manifest) {
    this.root = root;
    this.manifest = manifest;
  }

  addArtifact(logicalName, bytes, kind = "log") {
    if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/.test(logicalName)) throw new Error("invalid artifact name");
    const redacted = kind === "log" || kind === "junit" ? Buffer.from(redactText(Buffer.from(bytes).toString("utf8"))) : Buffer.from(bytes);
    const digest = sha256(redacted);
    const existing = this.manifest.artifacts.find((artifact) => artifact.name === logicalName);
    if (existing && existing.sha256 !== digest) throw new Error("artifact resume hash mismatch");
    writeFileSync(join(this.root, "artifacts", logicalName), redacted);
    if (!existing) this.manifest.artifacts.push({ name: logicalName, kind, sha256: digest, sizeBytes: redacted.length });
    writeManifest(this.root, this.manifest);
  }

  finalize(finishedAt) {
    this.manifest.status = "complete";
    this.manifest.finishedAt = finishedAt;
    this.manifest.lifecycle.push({ event: "collection_completed", at: finishedAt });
    writeManifest(this.root, this.manifest);
    return structuredClone(this.manifest);
  }
}

export function verifyEvidenceBundle(root) {
  const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8"));
  const failures = [];
  if (manifest.status !== "complete") failures.push("bundle_incomplete");
  for (const artifact of manifest.artifacts ?? []) {
    const path = join(root, "artifacts", artifact.name);
    if (!existsSync(path)) failures.push(`missing:${artifact.name}`);
    else if (sha256(readFileSync(path)) !== artifact.sha256 || statSync(path).size !== artifact.sizeBytes) failures.push(`hash_or_size_mismatch:${artifact.name}`);
  }
  return { valid: failures.length === 0, failures, manifest };
}

function prepareCommand(platform, runId, expectedHash) {
  if (platform === "windows") {
    return `$root='.desktoplab-remote/${runId}'; New-Item -ItemType Directory -Force -Path \"$root/source\" | Out-Null; tar.exe -xf '.desktoplab-remote/${runId}.tar' -C \"$root/source\"; $hash=(Get-FileHash '.desktoplab-remote/${runId}.tar' -Algorithm SHA256).Hash.ToLower(); Write-Output \"DESKTOPLAB_ARCHIVE_SHA256=$hash\"`;
  }
  return `mkdir -p '.desktoplab-remote/${runId}/source' && tar -xf '.desktoplab-remote/${runId}.tar' -C '.desktoplab-remote/${runId}/source' && printf 'DESKTOPLAB_ARCHIVE_SHA256=%s\\n' "$(sha256sum '.desktoplab-remote/${runId}.tar' | cut -d ' ' -f 1)"`;
}

function incomplete(snapshot, target, state, reason) {
  return { kind: "desktoplab.remote-sync", schemaVersion: 1, status: "incomplete", state, reason, target: targetFingerprint(target), source: sourceIdentity(snapshot) };
}

function sourceIdentity(snapshot) {
  return { commit: snapshot.commit, treeState: snapshot.treeState, archiveSha256: snapshot.archiveSha256, archiveBytes: snapshot.archiveBytes };
}

function targetFingerprint(target) {
  return { id: target.id, platform: target.platform, architecture: target.architecture, trustLevel: target.trustLevel };
}

function writeManifest(root, manifest) {
  const path = join(root, "manifest.json");
  const temporary = join(root, `.manifest-${process.pid}.tmp`);
  writeFileSync(temporary, `${JSON.stringify(manifest, null, 2)}\n`, { mode: 0o600 });
  renameSync(temporary, path);
}

function redactText(value) {
  return value
    .replaceAll(resolve(os.homedir()), "[HOME]")
    .replace(/[A-Z]:\\Users\\[^\\\s]+/gi, "[HOME]")
    .replace(/\b(token|password|secret|api[_-]?key)\s*[=:]\s*\S+/gi, "$1=[REDACTED]");
}

function git(root, args, encoding) {
  const result = spawnSync("git", args, { cwd: root, encoding, maxBuffer: 128 * 1024 * 1024, shell: false });
  if (result.error || result.status !== 0) throw new Error(result.error?.message ?? String(result.stderr));
  return result;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
