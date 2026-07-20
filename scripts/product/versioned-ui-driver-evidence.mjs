import { createHash } from "node:crypto";
import { readFileSync, realpathSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

export function versionedUiDriverFailures(record, repoRoot) {
  const failures = [];
  if (!record || record.technology !== "macos_native_accessibility") return ["native UI driver provenance missing"];
  if (record.keyboardTechnology !== "macos_system_keyboard_events") failures.push("native UI driver keyboard provenance missing");
  const dependencies = Array.isArray(record.dependencies) ? record.dependencies : [];
  if (dependencies.length === 0) failures.push("native UI driver dependencies missing");
  const sources = [{ path: record.path, sha256: record.sha256 }, ...dependencies];
  const productRoot = canonicalDirectory(join(repoRoot, "scripts/product"));
  const driverRoot = canonicalDirectory(join(repoRoot, "scripts/product/drivers"));
  const seen = new Set();
  const verified = [];
  for (const source of sources) {
    const path = canonicalFile(source.path);
    if (!path || !productRoot || relative(productRoot, path).startsWith("..")) {
      failures.push("UI driver source is outside the versioned product boundary");
      continue;
    }
    if (seen.has(path)) failures.push("UI driver source dependency is duplicated");
    seen.add(path);
    const sha256 = digest(readFileSync(path));
    if (source.sha256 !== sha256) failures.push(`UI driver source hash mismatch: ${relative(repoRoot, path)}`);
    verified.push({ path, sha256 });
  }
  if (!driverRoot || !verified[0] || relative(driverRoot, verified[0].path).startsWith("..")) failures.push("versioned UI driver entrypoint missing");
  const bundle = digest(verified.map((entry) => `${relative(repoRoot, entry.path)}\0${entry.sha256}`).join("\0"));
  if (record.bundleSha256 !== bundle) failures.push("UI driver source bundle hash mismatch");
  return failures;
}

function canonicalFile(path) {
  try {
    const target = realpathSync(resolve(path));
    return statSync(target).isFile() ? target : null;
  } catch { return null; }
}

function canonicalDirectory(path) {
  try {
    const target = realpathSync(resolve(path));
    return statSync(target).isDirectory() ? target : null;
  } catch { return null; }
}

function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
