import { existsSync, realpathSync, statSync } from "node:fs";
import { join } from "node:path";

import { moduleSourceBundleDigest } from "./versioned-module-bundle.mjs";

const digestPattern = /^sha256:[a-f0-9]{64}$/i;

export function validateExecutorProvenance(provenance) {
  const failures = [];
  if (provenance?.kind !== "versioned_external_driver" || provenance?.schemaVersion !== 2) failures.push("versioned campaign executor provenance missing");
  if (!nonEmpty(provenance?.id)) failures.push("campaign executor id missing");
  if (!digestPattern.test(provenance?.sha256 ?? "")) failures.push("campaign executor digest missing");
  const sources = validExecutorSources(provenance?.sources);
  if (!digestPattern.test(provenance?.bundleSha256 ?? "") || sources.length === 0) failures.push("campaign executor bundle missing");
  if (provenance?.sourceCount !== sources.length || provenance?.sourceCount < 1) failures.push("campaign executor source count invalid");
  if (sources.length > 0 && provenance?.bundleSha256 !== moduleSourceBundleDigest(sources)) failures.push("campaign executor bundle digest invalid");
  if (!sources.some((source) => source.path.split("/").at(-1) === provenance?.id && source.sha256 === provenance?.sha256)) failures.push("campaign executor entry source missing");
  return failures;
}

export function sanitizeExecutorProvenance(provenance) {
  return {
    kind: provenance?.kind ?? null,
    schemaVersion: provenance?.schemaVersion ?? null,
    id: safeIdentifier(provenance?.id),
    sha256: safeDigest(provenance?.sha256),
    bundleSha256: safeDigest(provenance?.bundleSha256),
    sourceCount: Number.isInteger(provenance?.sourceCount) ? provenance.sourceCount : null,
    sources: validExecutorSources(provenance?.sources),
  };
}

export function normalizeIsolation(isolation) {
  return {
    workspaceId: isolation?.workspaceId,
    workspacePath: canonicalPath(isolation?.workspacePath),
    sessionId: isolation?.sessionId,
    statePath: canonicalPath(isolation?.statePath),
  };
}

export function sanitizedIsolation(isolation, digest) {
  return {
    workspaceId: safeIdentifier(isolation?.workspaceId),
    workspacePath: isolation?.workspacePath ? digest(isolation.workspacePath) : null,
    sessionId: safeIdentifier(isolation?.sessionId),
    statePath: isolation?.statePath ? digest(isolation.statePath) : null,
  };
}

export function validateRunEvidence(descriptor, output, isolation) {
  const failures = [];
  if (!isolation.workspacePath || !isDirectory(isolation.workspacePath) || !existsSync(join(isolation.workspacePath, ".git"))) failures.push("workspace isolation path is not a real Git repository");
  if (!isolation.statePath || !isFile(isolation.statePath)) failures.push("state isolation path is not a real file");
  if (output?.trace?.sessionId !== isolation.sessionId) failures.push("trace session does not match isolation session");
  const provenance = output?.provenance;
  if (provenance?.executionKind !== "installed_app_ui") failures.push("installed app UI provenance missing");
  if (provenance?.candidateId !== descriptor.candidateId) failures.push("run provenance belongs to another candidate");
  if (provenance?.appHash !== descriptor.appHash) failures.push("run provenance belongs to another app payload");
  if (!Number.isInteger(provenance?.modelRequestCount) || provenance.modelRequestCount < 1) failures.push("real model request provenance missing");
  if (provenance?.testControlRequests !== 0) failures.push("test-control execution is forbidden");
  if (!digestPattern.test(provenance?.uiDriverSha256 ?? "")) failures.push("versioned UI driver provenance missing");
  if (!digestPattern.test(provenance?.interactionSha256 ?? "")) failures.push("UI interaction provenance missing");
  if (!digestPattern.test(provenance?.screenshotSha256 ?? "")) failures.push("UI screenshot provenance missing");
  return failures;
}

export function sanitizedProvenance(provenance) {
  return {
    executionKind: provenance?.executionKind ?? null,
    appHash: safeDigest(provenance?.appHash),
    modelRequestCount: Number.isInteger(provenance?.modelRequestCount) ? provenance.modelRequestCount : null,
    testControlRequests: Number.isInteger(provenance?.testControlRequests) ? provenance.testControlRequests : null,
    uiDriverSha256: safeDigest(provenance?.uiDriverSha256),
    interactionSha256: safeDigest(provenance?.interactionSha256),
    screenshotSha256: safeDigest(provenance?.screenshotSha256),
  };
}

function canonicalPath(value) {
  if (!nonEmpty(value)) return null;
  try { return realpathSync(value); } catch { return null; }
}

function isDirectory(path) {
  try { return statSync(path).isDirectory(); } catch { return false; }
}

function isFile(path) {
  try { return statSync(path).isFile(); } catch { return false; }
}

function safeDigest(value) {
  return digestPattern.test(value ?? "") ? value.toLowerCase() : null;
}

function safeIdentifier(value) {
  return nonEmpty(value) && /^[a-zA-Z0-9._-]{1,128}$/.test(value) ? value : null;
}

function validExecutorSources(value) {
  if (!Array.isArray(value)) return [];
  const sources = value.filter((source) => safeSourcePath(source?.path) && digestPattern.test(source?.sha256 ?? ""))
    .map((source) => ({ path: source.path, sha256: source.sha256.toLowerCase() }));
  return sources.length === value.length && new Set(sources.map((source) => source.path)).size === sources.length ? sources : [];
}

function safeSourcePath(value) {
  return nonEmpty(value) && !value.startsWith("/") && !value.includes("\\")
    && value.split("/").every((part) => part !== "" && part !== "." && part !== "..")
    && /^[a-zA-Z0-9._/-]+$/.test(value);
}

function nonEmpty(value) {
  return typeof value === "string" && value.trim().length > 0;
}
