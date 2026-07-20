import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { assessExternalReferencePolicy } from "./external-reference-guard-core.mjs";

const manifestCandidates = [
  "Cargo.toml",
  "Cargo.lock",
  "package.json",
  "package-lock.json",
  "apps/desktop/package.json",
  "apps/desktop/src-tauri/Cargo.toml",
  "apps/desktop/src-tauri/Cargo.lock",
];
const mode = parseMode(process.argv.slice(2));
const ledgerPath = "docs/evidence/openclaw-ecosystem-reference-ledger.json";

const trackedFiles = execFileSync("git", ["ls-files"], { encoding: "utf8" }).split(/\r?\n/).filter(Boolean);
const manifestSources = manifestCandidates
  .filter((path) => existsSync(path))
  .map((path) => ({ path, source: readFileSync(path, "utf8") }));
const ledger = existsSync(ledgerPath) ? JSON.parse(readFileSync(ledgerPath, "utf8")) : null;
const failures = assessExternalReferencePolicy({
  ignoreSource: readFileSync(".gitignore", "utf8"),
  trackedFiles,
  manifestSources,
  ledger,
  requireLedger: mode === "internal",
});

if (failures.length > 0) {
  console.error("External reference guard failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(
  ledger
    ? `External reference guard passed: ${ledger.references.length} pinned research references, zero build inputs.`
    : "External reference guard passed: private research ledger omitted, zero tracked references or build inputs.",
);

function parseMode(args) {
  const modeIndex = args.indexOf("--mode");
  const mode = modeIndex >= 0 ? args[modeIndex + 1] : "internal";
  if (!new Set(["internal", "public-export"]).has(mode)) {
    console.error(`Invalid mode: ${mode}`);
    process.exit(2);
  }
  return mode;
}
