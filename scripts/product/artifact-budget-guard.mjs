import { existsSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { buildCacheRoots, expandedReleaseApps } from "./build-cache-policy.mjs";

const ARTIFACT_DIRS = [
  "apps/desktop/test-artifacts",
  "apps/desktop/test-results",
  "apps/desktop/playwright-report",
];

const IMAGE_EXTENSIONS = new Set([".png", ".jpg", ".jpeg", ".webp", ".heic"]);
const MAX_ARTIFACT_BYTES = readIntEnv("DESKTOPLAB_ARTIFACT_BUDGET_BYTES", 256 * 1024 * 1024);
const MAX_IMAGE_BYTES = readIntEnv("DESKTOPLAB_SCREENSHOT_BUDGET_BYTES", 128 * 1024 * 1024);
const MAX_IMAGE_COUNT = readIntEnv("DESKTOPLAB_SCREENSHOT_BUDGET_COUNT", 500);
const MAX_SINGLE_IMAGE_BYTES = readIntEnv("DESKTOPLAB_SCREENSHOT_MAX_BYTES", 20 * 1024 * 1024);
const MAX_BUILD_CACHE_BYTES = readIntEnv("DESKTOPLAB_BUILD_CACHE_BUDGET_BYTES", 5 * 1024 * 1024 * 1024);

const records = [];
for (const dir of ARTIFACT_DIRS) {
  collect(dir, records);
}

const cacheRecords = [];
for (const entryPath of [...buildCacheRoots(process.cwd()), ...expandedReleaseApps(process.cwd())]) {
  collect(entryPath, cacheRecords);
}

const imageRecords = records.filter((record) => IMAGE_EXTENSIONS.has(path.extname(record.path).toLowerCase()));
const artifactBytes = sum(records.map((record) => record.bytes));
const buildCacheBytes = sum(cacheRecords.map((record) => record.bytes));
const imageBytes = sum(imageRecords.map((record) => record.bytes));
const oversizedImages = imageRecords.filter((record) => record.bytes > MAX_SINGLE_IMAGE_BYTES);
const failures = [];

if (artifactBytes > MAX_ARTIFACT_BYTES) {
  failures.push(`artifact directories use ${formatBytes(artifactBytes)}, budget is ${formatBytes(MAX_ARTIFACT_BYTES)}`);
}

if (imageBytes > MAX_IMAGE_BYTES) {
  failures.push(`screenshots/images use ${formatBytes(imageBytes)}, budget is ${formatBytes(MAX_IMAGE_BYTES)}`);
}

if (imageRecords.length > MAX_IMAGE_COUNT) {
  failures.push(`screenshots/images count is ${imageRecords.length}, budget is ${MAX_IMAGE_COUNT}`);
}

for (const record of oversizedImages) {
  failures.push(`${record.path} is ${formatBytes(record.bytes)}, per-image budget is ${formatBytes(MAX_SINGLE_IMAGE_BYTES)}`);
}

const buildCacheOverBudget = buildCacheBytes > MAX_BUILD_CACHE_BYTES;
if (buildCacheOverBudget) {
  failures.push(`build caches use ${formatBytes(buildCacheBytes)}, budget is ${formatBytes(MAX_BUILD_CACHE_BYTES)}`);
}

if (failures.length > 0) {
  console.error("Artifact budget guard failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  console.error("\nLargest artifact files:");
  for (const record of records.sort((a, b) => b.bytes - a.bytes).slice(0, 10)) {
    console.error(`- ${formatBytes(record.bytes)} ${record.path}`);
  }
  if (cacheRecords.length > 0) {
    console.error("\nLargest build cache files:");
    for (const record of cacheRecords.sort((a, b) => b.bytes - a.bytes).slice(0, 10)) {
      console.error(`- ${formatBytes(record.bytes)} ${record.path}`);
    }
  }
  console.error("\nArchive stale visual evidence or delete regenerable build caches before rerunning the product gate.");
  process.exit(1);
}

console.log(
  `Artifact budget guard passed: ${formatBytes(artifactBytes)} artifacts, ${imageRecords.length} image(s), ${formatBytes(imageBytes)} images, ${formatBytes(buildCacheBytes)} build cache.`,
);

function collect(entryPath, records) {
  if (!existsSync(entryPath)) return;
  const stats = statSync(entryPath);
  if (stats.isFile()) {
    records.push({ path: entryPath, bytes: stats.size });
    return;
  }
  if (!stats.isDirectory()) return;
  for (const child of readdirSync(entryPath)) {
    collect(path.join(entryPath, child), records);
  }
}

function sum(values) {
  return values.reduce((total, value) => total + value, 0);
}

function readIntEnv(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return value;
}

function formatBytes(bytes) {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)}${units[unitIndex]}`;
}
