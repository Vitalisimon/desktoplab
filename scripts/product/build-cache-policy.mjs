import { existsSync, lstatSync, readdirSync } from "node:fs";
import path from "node:path";

export const BUILD_CACHE_RELATIVE_PATHS = [
  "node_modules",
  "target",
  "apps/desktop/dist",
  "apps/desktop/src-tauri/target",
  "dist/public-export",
];

const PUBLICATION_RELATIVE_ROOTS = [
  "dist/public-publication",
  "dist/public-publish",
];

export function buildCacheRoots(root) {
  return [root, ...publicationCheckouts(root)].flatMap((workspaceRoot) =>
    [
      ...BUILD_CACHE_RELATIVE_PATHS.map((entry) => path.join(workspaceRoot, entry)),
      ...legacyTypeScriptBuildInfo(workspaceRoot),
    ]
  );
}

export function expandedReleaseApps(root) {
  const releaseRoot = path.join(root, "dist", "release", "macos");
  if (!isDirectory(releaseRoot)) return [];

  return readdirSync(releaseRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && !entry.isSymbolicLink())
    .map((entry) => path.join(releaseRoot, entry.name, "DesktopLab.app"))
    .filter(isDirectory);
}

export function archivedExpandedReleaseApps(root) {
  return expandedReleaseApps(root).filter((appPath) => {
    const releaseDir = path.dirname(appPath);
    return readdirSync(releaseDir, { withFileTypes: true }).some((entry) =>
      entry.isFile()
      && (entry.name.endsWith(".dmg") || entry.name === "DesktopLab.app.zip")
    );
  });
}

function publicationCheckouts(root) {
  return PUBLICATION_RELATIVE_ROOTS.flatMap((relativeRoot) => {
    const publicationRoot = path.join(root, relativeRoot);
    if (!isDirectory(publicationRoot)) return [];
    return readdirSync(publicationRoot, { withFileTypes: true })
      .filter((entry) => entry.isDirectory() && !entry.isSymbolicLink())
      .map((entry) => path.join(publicationRoot, entry.name));
  });
}

function legacyTypeScriptBuildInfo(root) {
  const frontendRoot = path.join(root, "apps", "desktop");
  if (!isDirectory(frontendRoot)) return [];
  return readdirSync(frontendRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && !entry.isSymbolicLink() && entry.name.endsWith(".tsbuildinfo"))
    .map((entry) => path.join(frontendRoot, entry.name));
}

function isDirectory(entryPath) {
  if (!existsSync(entryPath)) return false;
  const stats = lstatSync(entryPath);
  return !stats.isSymbolicLink() && stats.isDirectory();
}
