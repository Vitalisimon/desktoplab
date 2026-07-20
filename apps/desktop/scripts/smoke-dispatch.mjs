import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { cleanupGeneratedArtifactsAfterPass, resolvePlaywrightCliPath } from "./smoke-dispatch-lib.mjs";

const args = process.argv.slice(2);
const appDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const workspaceRoot = path.resolve(appDir, "../..");
const command =
  args.length > 0
    ? ["node", resolvePlaywrightCliPath({ appDir, workspaceRoot }), "test", ...args]
    : ["npm", "run", "smoke:product"];
const result = spawnSync(command[0], command.slice(1), { stdio: "inherit", shell: process.platform === "win32" });
const status = result.status ?? 1;
const cleanupStatus = cleanupGeneratedArtifactsAfterPass({ appDir, status });

if (cleanupStatus === "cleaned") {
  console.log("DesktopLab smoke artifacts cleaned after passing run.");
} else if (cleanupStatus === "preserved_by_env") {
  console.log("DesktopLab smoke artifacts preserved by DESKTOPLAB_KEEP_TEST_ARTIFACTS=1.");
}

process.exit(status);
