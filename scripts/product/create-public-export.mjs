import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { sha256File } from "../security/supply-chain-evidence-core.mjs";

const repoRoot = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8"
}).trim();
const sourceCommit = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8"
}).trim();
const sourceTreeState = execFileSync("git", ["status", "--porcelain=v1"], {
  cwd: repoRoot,
  encoding: "utf8"
}).trim() ? "dirty" : "clean";
const lockfiles = ["Cargo.lock", "package-lock.json", "apps/desktop/src-tauri/Cargo.lock"].map((path) => ({
  path,
  sha256: sha256File(join(repoRoot, path))
}));

const exportRoot = join(repoRoot, "dist", "public-export", "desktoplab");
const trackedFiles = execFileSync("git", ["ls-files", "-z"], {
  cwd: repoRoot
})
  .toString("utf8")
  .split("\0")
  .filter(Boolean)
  .sort();

const excludedExact = new Set([
  "AGENTS.md",
  "PUBLIC_EXPORT_MANIFEST.json"
]);

const excludedPrefixes = [
  "docs/",
  "dist/",
  "target/",
  "node_modules/",
  "apps/desktop/test-artifacts/",
  "apps/desktop/test-results/",
  "apps/desktop/playwright-report/"
];

const excludedSuffixes = [
  ".dmg",
  ".msi",
  ".AppImage",
  ".deb",
  ".rpm",
  ".tsbuildinfo"
];

function isExcluded(file) {
  return (
    excludedExact.has(file) ||
    excludedPrefixes.some((prefix) => file.startsWith(prefix)) ||
    excludedSuffixes.some((suffix) => file.endsWith(suffix)) ||
    file === ".env" ||
    file.startsWith(".env.")
  );
}

const included = [];
const excluded = [];

rmSync(exportRoot, { force: true, recursive: true });
mkdirSync(exportRoot, { recursive: true });

for (const file of trackedFiles) {
  if (isExcluded(file)) {
    excluded.push(file);
    continue;
  }

  const destination = join(exportRoot, file);
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(join(repoRoot, file), destination);
  included.push(file);
}

writeFileSync(
  join(exportRoot, "PUBLIC_EXPORT_MANIFEST.json"),
  `${JSON.stringify(
    {
      product: "DesktopLab",
      sourceCommit,
      sourceTreeState,
      lockfiles,
      generatedAt: new Date().toISOString(),
      historyPolicy: "clean-public-history-required",
      directVisibilityOfSourceRepo: "blocked",
      includedCount: included.length,
      excluded
    },
    null,
    2
  )}\n`
);

console.log(`Public export written to ${exportRoot}`);
console.log(`Included files: ${included.length}`);
console.log(`Excluded files: ${excluded.length}`);
