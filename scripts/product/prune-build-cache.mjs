import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { archivedExpandedReleaseApps, buildCacheRoots, expandedReleaseApps } from "./build-cache-policy.mjs";

const root = path.resolve(process.cwd());
const budgetBytes = readPositiveInt("DESKTOPLAB_BUILD_CACHE_BUDGET_BYTES", 5 * 1024 * 1024 * 1024);
const intervalMs = readPositiveInt("DESKTOPLAB_BUILD_CACHE_PRUNE_INTERVAL_MS", 7 * 24 * 60 * 60 * 1000);
const markerPath = path.resolve(
  process.env.DESKTOPLAB_BUILD_CACHE_MARKER ?? path.join(root, "dist", "cache-maintenance.json"),
);
const now = new Date();
const cacheRoots = buildCacheRoots(root);
const releaseApps = expandedReleaseApps(root);
const archivedReleaseApps = archivedExpandedReleaseApps(root);
const cacheBytes = [...cacheRoots, ...releaseApps].reduce((total, entry) => total + entryBytes(entry), 0);
const previous = readMarker(markerPath);
const previousMaintenance = Date.parse(previous?.lastMaintenanceAt ?? previous?.lastPrunedAt ?? "");
const intervalElapsed = Number.isFinite(previousMaintenance)
  && now.getTime() - previousMaintenance >= intervalMs;
const reason = cacheBytes > budgetBytes
  ? "over_budget"
  : intervalElapsed && cacheBytes > 0
    ? "interval_elapsed"
    : archivedReleaseApps.length > 0
      ? "expanded_release_residue"
      : null;

if (reason) {
  const entriesToRemove = reason === "expanded_release_residue"
    ? archivedReleaseApps
    : [...cacheRoots, ...archivedReleaseApps];
  for (const entry of entriesToRemove) {
    rmSync(entry, {
      force: true,
      recursive: true,
      maxRetries: 5,
      retryDelay: 200,
    });
  }
  const cacheBytesAfter = [...cacheRoots, ...expandedReleaseApps(root)]
    .reduce((total, entry) => total + entryBytes(entry), 0);
  writeMarker({
    schemaVersion: 2,
    status: "pruned",
    reason,
    cacheBytesBefore: cacheBytes,
    cacheBytesAfter,
    expandedReleaseAppsPruned: archivedReleaseApps.length,
    budgetBytes,
    lastMaintenanceAt: now.toISOString(),
    lastPrunedAt: now.toISOString(),
  });
  console.log(`Build cache maintenance pruned ${formatBytes(cacheBytes - cacheBytesAfter)} (${reason}).`);
  if (cacheBytesAfter > budgetBytes) {
    console.error(`Build cache maintenance left ${formatBytes(cacheBytesAfter)}, above ${formatBytes(budgetBytes)} budget.`);
    process.exit(1);
  }
} else {
  writeMarker({
    schemaVersion: 2,
    status: "observed",
    reason: cacheBytes === 0 ? "empty" : "within_budget_and_interval",
    cacheBytesBefore: cacheBytes,
    cacheBytesAfter: cacheBytes,
    expandedReleaseAppsPruned: 0,
    budgetBytes,
    lastMaintenanceAt: previous?.lastMaintenanceAt ?? now.toISOString(),
    lastPrunedAt: previous?.lastPrunedAt ?? null,
    lastObservedAt: now.toISOString(),
  });
  console.log(`Build cache maintenance kept ${formatBytes(cacheBytes)} within policy.`);
}

function entryBytes(entryPath) {
  if (!existsSync(entryPath)) return 0;
  const stats = lstatSync(entryPath);
  if (stats.isSymbolicLink() || stats.isFile()) return stats.size;
  if (!stats.isDirectory()) return 0;
  return readdirSync(entryPath).reduce(
    (total, child) => total + entryBytes(path.join(entryPath, child)),
    0,
  );
}

function readMarker(entryPath) {
  if (!existsSync(entryPath)) return null;
  try {
    return JSON.parse(readFileSync(entryPath, "utf8"));
  } catch {
    return null;
  }
}

function writeMarker(payload) {
  mkdirSync(path.dirname(markerPath), { recursive: true });
  const temporary = `${markerPath}.${process.pid}.tmp`;
  writeFileSync(temporary, `${JSON.stringify(payload, null, 2)}\n`, { mode: 0o600 });
  renameSync(temporary, markerPath);
}

function readPositiveInt(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value) || value <= 0) throw new Error(`${name} must be a positive integer`);
  return value;
}

function formatBytes(bytes) {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 1)}${units[unit]}`;
}
