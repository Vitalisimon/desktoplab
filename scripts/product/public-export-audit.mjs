import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { hostname } from "node:os";
import { join } from "node:path";
import { scanText, sha256File } from "../security/supply-chain-evidence-core.mjs";
import {
  buildDynamicForbiddenPatterns,
  decodeTextCandidate,
} from "./public-export-content-scan.mjs";
import {
  directSourceAuditRequired,
  isForbiddenPublicTrackedPath,
} from "./public-export-audit-policy.mjs";

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8"
}).trim();
const directSourceMode = directSourceAuditRequired({
  args: process.argv.slice(2),
  repository: process.env.GITHUB_REPOSITORY,
});
execFileSync("node", ["scripts/product/create-public-export.mjs"], {
  cwd: repoRoot,
  encoding: "utf8",
  stdio: ["ignore", "ignore", "inherit"]
});

function git(args) {
  return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" });
}

const trackedFiles = git(["ls-files", "-z"])
  .split("\0")
  .filter(Boolean)
  .sort();
const historyFiles = git(["log", "--all", "--name-only", "--pretty=format:"])
  .split("\n")
  .map((line) => line.trim())
  .filter(Boolean);

const forbiddenTracked = trackedFiles.filter(isForbiddenPublicTrackedPath);

const privateDocsInCurrentTree = trackedFiles.filter((file) => file.startsWith("docs/"));
const privateDocsInHistory = [...new Set(historyFiles.filter((file) => file.startsWith("docs/")))].sort();
const exportRoot = join(repoRoot, "dist", "public-export", "desktoplab");
const exportFindings = [];
const expectedCommit = git(["rev-parse", "HEAD"]).trim();
const expectedLockfiles = ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"].map((path) => ({
  path,
  sha256: sha256File(join(repoRoot, path))
}));

const dynamicForbiddenText = buildDynamicForbiddenPatterns({
  home: process.env.HOME,
  hostname: hostname(),
  blocklist: process.env.DESKTOPLAB_PUBLIC_EXPORT_BLOCKLIST,
});

function walk(root, prefix = "") {
  if (!existsSync(root)) {
    return [];
  }

  const files = [];
  for (const entry of readdirSync(root)) {
    const fullPath = join(root, entry);
    const relative = prefix ? `${prefix}/${entry}` : entry;
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      files.push(...walk(fullPath, relative));
    } else {
      files.push(relative);
    }
  }
  return files.sort();
}

const exportFiles = walk(exportRoot);
const allowedSensitiveKinds = new Map([
  ["crates/desktoplab-redaction/tests/redaction_patterns.rs", ["private-key"]],
  ["crates/desktoplab-tool-gateway/tests/test_runner.rs", ["openai-secret"]]
]);
const forbiddenPrivateResearchFiles = [
  "docs/installed-competitor-app-bundle-audit.md",
  "docs/24.8-native-integration-competitor-learnings-plan.md"
];
for (const file of exportFiles) {
  if (
    file === "AGENTS.md" ||
    file.startsWith("docs/") ||
    file.startsWith(".env") ||
    file.startsWith("dist/") ||
    file.startsWith("target/") ||
    file.includes("test-artifacts/") ||
    file.includes("test-results/") ||
    file === ".git" ||
    file.startsWith(".git/") ||
    /\.(dmg|msi|AppImage|deb|rpm)$/.test(file)
  ) {
    exportFindings.push(`forbidden export path: ${file}`);
  }
  if (forbiddenPrivateResearchFiles.includes(file)) {
    exportFindings.push(`private research leaked: ${file}`);
  }

  const fullPath = join(exportRoot, file);
  if (statSync(fullPath).size <= 512_000) {
    const text = decodeTextCandidate(readFileSync(fullPath));
    if (text === null) {
      continue;
    }
    for (const pattern of dynamicForbiddenText) {
      if (pattern.test(text)) {
        exportFindings.push(`private wording in ${file}: ${pattern}`);
      }
    }
    for (const finding of scanText({
      label: file,
      text,
      privateValues: [],
      allowedKinds: allowedSensitiveKinds.get(file) ?? []
    })) {
      exportFindings.push(`sensitive content in ${file}: ${finding.kind}`);
    }
    if (/\b(?:10\.(?:\d{1,3}\.){2}\d{1,3}|192\.168\.(?:\d{1,3}\.)\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.(?:\d{1,3}\.)\d{1,3})\b/.test(text)) {
      exportFindings.push(`private network address in ${file}`);
    }
  }
}

const exportManifestPath = join(exportRoot, "PUBLIC_EXPORT_MANIFEST.json");
const exportManifest = existsSync(exportManifestPath) ? JSON.parse(readFileSync(exportManifestPath, "utf8")) : null;
if (exportManifest?.sourceCommit !== expectedCommit) exportFindings.push("export manifest commit does not match HEAD");
if (exportManifest?.sourceTreeState !== "clean") exportFindings.push("export was generated from a dirty source tree");
for (const lock of expectedLockfiles) {
  const recorded = exportManifest?.lockfiles?.find((item) => item.path === lock.path);
  if (recorded?.sha256 !== lock.sha256) exportFindings.push(`export lock hash mismatch: ${lock.path}`);
}

const result = {
  sourceTree: {
    trackedFiles: trackedFiles.length,
    directPublicVisibility: currentTreeVisibility(forbiddenTracked, privateDocsInCurrentTree),
    forbiddenTracked,
    privateDocsInCurrentTree
  },
  sourceHistory: {
    directPublicVisibility: privateDocsInHistory.length > 0 ? "blocked" : "allowed",
    privateDocsInHistoryCount: privateDocsInHistory.length
  },
  publicExportCandidate: existsSync(exportRoot)
    ? {
        path: exportRoot,
        sourceCommit: exportManifest?.sourceCommit ?? null,
        sourceTreeState: exportManifest?.sourceTreeState ?? null,
        lockfiles: exportManifest?.lockfiles ?? [],
        fileCount: exportFiles.length,
        findings: exportFindings
      }
    : {
        path: exportRoot,
        fileCount: 0,
        findings: ["export candidate not generated"]
      }
};

for (const file of forbiddenPrivateResearchFiles) {
  if (trackedFiles.includes(file)) {
    result.sourceTree.forbiddenTracked.push(file);
  }
}

console.log(JSON.stringify(result, null, 2));

const currentTreeUnsafe = forbiddenTracked.length > 0 || privateDocsInCurrentTree.length > 0;
const exportMissing = !existsSync(exportRoot);
const exportUnsafe = exportMissing || exportFindings.length > 0;

if (exportUnsafe || (directSourceMode && currentTreeUnsafe)) {
  process.exit(1);
}

function currentTreeVisibility(forbiddenPaths, privateDocs) {
  return forbiddenPaths.length > 0 || privateDocs.length > 0 ? "blocked_use_historyless_export" : "allowed";
}
