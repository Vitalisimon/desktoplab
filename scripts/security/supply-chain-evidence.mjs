#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildCycloneDx, classifyAuditAdvisories, classifyLicenses, scanText, sha256File } from "./supply-chain-evidence-core.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const outputDir = join(root, "dist/release/supply-chain");
const exportRoot = join(root, "dist/public-export/desktoplab");
const artifactManifestPath = join(root, "dist/desktoplab-packaging/artifact-manifest.json");
const allowDirty = process.argv.includes("--allow-dirty");
const commit = git(["rev-parse", "HEAD"]);
const dirty = git(["status", "--porcelain=v1"]).length > 0;
const lockPaths = ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"];
const lockHashes = lockPaths.map((path) => ({ path, sha256: sha256File(join(root, path)) }));

if (dirty && !allowDirty) throw new Error("supply-chain evidence requires a clean source tree");
mkdirSync(outputDir, { recursive: true });

const cargoMetadata = jsonCommand("cargo", ["metadata", "--locked", "--format-version", "1"]);
const cargoAudits = [
  { scope: "workspace", report: jsonCommand("cargo", ["audit", "--json"]) },
  {
    scope: "tauri",
    report: jsonCommand("cargo", ["audit", "--file", "apps/desktop/src-tauri/Cargo.lock", "--json"]),
  },
];
const npmAudit = jsonCommand("npm", ["audit", "--json"]);
const npmTree = jsonCommand("npm", ["ls", "--all", "--json"]);
const cargoTree = textCommand("cargo", ["tree", "--workspace", "--locked"]);
const diagnostics = JSON.parse(textCommand("cargo", ["run", "--quiet", "-p", "desktoplab-smoke-cli", "--example", "diagnostics_export"]));

execFileSync("node", ["scripts/security/scan-tracked-secrets.mjs"], { cwd: root, stdio: "pipe" });
execFileSync("node", ["scripts/product/create-public-export.mjs"], { cwd: root, stdio: "pipe" });

const cargoPackages = cargoMetadata.packages.map((pkg) => ({
  ecosystem: "cargo",
  name: pkg.name,
  version: pkg.version,
  license: pkg.license,
  source: pkg.source ?? "workspace",
}));
const npmPackages = npmInventory();
const licenses = classifyLicenses([...cargoPackages, ...npmPackages]);
const advisories = classifyAuditAdvisories({ cargoAudits, npmAudit });
const sourcePrivacy = scanTree(exportRoot, {
  privateValues: [process.env.HOME, root],
  allowedByPath: new Map([
    ["crates/desktoplab-redaction/tests/redaction_patterns.rs", ["private-key"]],
    ["crates/desktoplab-tool-gateway/tests/test_runner.rs", ["openai-secret"]],
  ]),
});
const artifact = inspectArtifact();
const diagnosticsPrivacy = scanText({
  label: "diagnostics-export",
  text: JSON.stringify(diagnostics),
  privateValues: [process.env.HOME, root],
});
const privateReporting = verifyPrivateReporting();
const tools = Object.fromEntries([
  ["node", textCommand("node", ["--version"])],
  ["npm", textCommand("npm", ["--version"])],
  ["cargo", textCommand("cargo", ["--version"])],
  ["rustc", textCommand("rustc", ["--version"])],
  ["cargoAudit", textCommand("cargo", ["audit", "--version"])],
]);

const checks = {
  sourceTree: { status: dirty ? "fail" : "pass", dirty },
  licenses: { status: licenses.status, packageCount: licenses.packageCount, findingCount: licenses.findings.length },
  advisories: { status: advisories.status, findingCount: advisories.findings.length },
  publicSourcePrivacy: { status: sourcePrivacy.length === 0 ? "pass" : "fail", findings: sourcePrivacy },
  artifact: artifact.summary,
  diagnosticsPrivacy: { status: diagnosticsPrivacy.length === 0 ? "pass" : "fail", findings: diagnosticsPrivacy },
};
const localStatus = Object.values(checks).every((check) => check.status === "pass") ? "pass" : "fail";
const status = localStatus === "pass" && privateReporting.status === "pass" ? "pass" : "blocked";
const tauriConfig = JSON.parse(readFileSync(join(root, "apps/desktop/src-tauri/tauri.conf.json"), "utf8"));
const report = {
  kind: "desktoplab.supply-chain-evidence",
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  status,
  localStatus,
  source: { commit, dirty, lockHashes },
  tools,
  checks,
  privateReporting,
  advisoryDatabases: Object.fromEntries(cargoAudits.map(({ scope, report }) => [scope, report.database])),
  dependencyCounts: { cargo: cargoPackages.length, npm: npmPackages.length },
  artifact: artifact.detail,
};
const sbom = buildCycloneDx({ commit, version: tauriConfig.version, cargoMetadata, npmPackages, lockHashes });

writeJson("evidence.json", report);
writeJson("licenses.json", licenses);
writeJson("advisories.json", advisories);
for (const { scope, report } of cargoAudits) writeJson(`cargo-audit-${scope}.json`, report);
writeJson("npm-audit.json", npmAudit);
writeJson("npm-dependency-tree.json", npmTree);
writeJson("sbom.cdx.json", sbom);
writeJson("diagnostics-export.json", diagnostics);
writeFileSync(join(outputDir, "cargo-dependency-tree.txt"), `${cargoTree}\n`);
console.log(JSON.stringify(report, null, 2));
if (localStatus !== "pass") process.exitCode = 1;

function npmInventory() {
  const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
  const packages = [];
  for (const [packagePath, entry] of Object.entries(lock.packages ?? {})) {
    if (entry.link) continue;
    const manifestPath = join(root, packagePath, "package.json");
    const manifest = existsSync(manifestPath) ? JSON.parse(readFileSync(manifestPath, "utf8")) : {};
    packages.push({
      ecosystem: "npm",
      name: manifest.name ?? entry.name ?? packagePath,
      version: manifest.version ?? entry.version,
      license: manifest.license ?? entry.license ?? null,
      source: packagePath || "workspace",
    });
  }
  return packages.sort((left, right) => `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`));
}

function inspectArtifact() {
  if (!existsSync(artifactManifestPath)) return artifactFailure("artifact manifest missing");
  const manifest = JSON.parse(readFileSync(artifactManifestPath, "utf8"));
  const findings = [];
  if (manifest.build?.commitSha !== commit) findings.push("artifact commit does not match source HEAD");
  if (manifest.build?.treeState !== "clean") findings.push("artifact was built from a dirty tree");
  for (const lock of lockHashes) {
    const recorded = manifest.build?.lockfiles?.find((item) => item.path === lock.path);
    if (recorded?.sha256 !== lock.sha256) findings.push(`artifact lock hash mismatch: ${lock.path}`);
  }
  const appEntry = manifest.entries?.find((entry) => entry.kind === "app_bundle");
  const appPath = appEntry ? join(root, appEntry.relativePath) : null;
  if (!appPath || !existsSync(appPath)) findings.push("app bundle missing");
  const privacyFindings = appPath ? scanArtifactBundle(appPath) : [];
  findings.push(...privacyFindings.map((finding) => `${finding.kind}: ${finding.label}`));
  const verify = spawnSync("node", ["scripts/packaging/verify-artifact-provenance.mjs"], { cwd: root, encoding: "utf8" });
  if (verify.status !== 0) findings.push("artifact provenance verification failed");
  return {
    summary: { status: findings.length === 0 ? "pass" : "fail", findings },
    detail: { manifest: relative(root, artifactManifestPath), build: manifest.build, entries: manifest.entries, privacyFindings },
  };
}

function scanArtifactBundle(appPath) {
  const findings = [];
  for (const file of walk(appPath)) {
    const buffer = readFileSync(file);
    const isBinary = buffer.subarray(0, 8_192).includes(0);
    const text = isBinary ? strings(file) : buffer.toString("utf8");
    findings.push(...scanText({ label: relative(root, file), text, privateValues: [process.env.HOME, root] }));
  }
  return findings;
}

function scanTree(directory, { privateValues, allowedByPath }) {
  const findings = [];
  for (const file of walk(directory)) {
    if (statSync(file).size > 2_000_000) continue;
    const buffer = readFileSync(file);
    if (buffer.subarray(0, 8_192).includes(0)) continue;
    const label = relative(directory, file);
    findings.push(...scanText({ label, text: buffer.toString("utf8"), privateValues, allowedKinds: allowedByPath.get(label) ?? [] }));
  }
  return findings;
}

function verifyPrivateReporting() {
  const reportUrl = process.env.DESKTOPLAB_PRIVATE_REPORT_TEST_URL;
  const match = reportUrl?.match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)\/security\/advisories\/(GHSA-[A-Za-z0-9-]+)$/);
  if (!match) return { status: "blocked", reason: "verified GitHub private vulnerability test report is required" };
  const endpoint = `repos/${match[1]}/${match[2]}/security-advisories/${match[3]}`;
  const result = spawnSync("gh", ["api", endpoint], { cwd: root, encoding: "utf8" });
  if (result.status !== 0) return { status: "blocked", reason: "private report could not be verified with the active GitHub account" };
  const advisory = JSON.parse(result.stdout);
  return { status: advisory.ghsa_id === match[3] ? "pass" : "blocked", repository: `${match[1]}/${match[2]}`, ghsaId: advisory.ghsa_id, state: advisory.state };
}

function walk(directory) {
  if (!existsSync(directory)) return [];
  const output = [];
  for (const entry of readdirSync(directory).sort()) {
    const path = join(directory, entry);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) continue;
    if (stat.isDirectory()) output.push(...walk(path));
    else if (stat.isFile()) output.push(path);
  }
  return output;
}

function strings(file) {
  const result = spawnSync("strings", [file], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  return result.status === 0 ? result.stdout : "";
}

function artifactFailure(reason) {
  return { summary: { status: "fail", findings: [reason] }, detail: null };
}

function jsonCommand(command, args) {
  return JSON.parse(textCommand(command, args));
}

function textCommand(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(`${command} ${args.join(" ")} failed: ${(result.stderr || result.stdout).trim()}`);
  return result.stdout.trim();
}

function git(args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function writeJson(name, value) {
  writeFileSync(join(outputDir, name), `${JSON.stringify(value, null, 2)}\n`);
}
