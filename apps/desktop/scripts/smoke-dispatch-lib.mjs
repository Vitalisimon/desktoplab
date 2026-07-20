import fs from "node:fs";
import path from "node:path";

const GENERATED_ARTIFACT_DIRS = [
  "test-artifacts",
  "test-results",
  "playwright-report",
];

export function resolvePlaywrightCliPath({
  appDir,
  workspaceRoot,
  exists = fs.existsSync,
}) {
  const candidates = [
    path.join(appDir, "node_modules/@playwright/test/cli.js"),
    path.join(workspaceRoot, "node_modules/@playwright/test/cli.js"),
  ];

  return candidates.find((candidate) => exists(candidate)) ?? candidates[0];
}

export function cleanupGeneratedArtifactsAfterPass({
  appDir,
  status,
  env = process.env,
  rm = fs.rmSync,
}) {
  if (status !== 0) return "preserved_failed_run";
  if (env.DESKTOPLAB_KEEP_TEST_ARTIFACTS === "1") return "preserved_by_env";

  for (const dir of GENERATED_ARTIFACT_DIRS) {
    rm(path.join(appDir, dir), { force: true, recursive: true });
  }
  return "cleaned";
}
