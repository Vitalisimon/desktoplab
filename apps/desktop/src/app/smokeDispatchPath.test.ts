import path from "node:path";
import { describe, expect, test } from "vitest";

import {
  cleanupGeneratedArtifactsAfterPass,
  resolvePlaywrightCliPath,
} from "../../scripts/smoke-dispatch-lib.mjs";

describe("smoke dispatcher playwright path", () => {
  const appDir = "/repo/apps/desktop";
  const workspaceRoot = "/repo";

  test("uses the app-local playwright install when present", () => {
    const appCli = path.join(appDir, "node_modules/@playwright/test/cli.js");

    expect(
      resolvePlaywrightCliPath({
        appDir,
        workspaceRoot,
        exists: (candidate) => candidate === appCli,
      }),
    ).toBe(appCli);
  });

  test("falls back to workspace-root playwright install on clean workspace installs", () => {
    const workspaceCli = path.join(workspaceRoot, "node_modules/@playwright/test/cli.js");

    expect(
      resolvePlaywrightCliPath({
        appDir,
        workspaceRoot,
        exists: (candidate) => candidate === workspaceCli,
      }),
    ).toBe(workspaceCli);
  });

  test("cleans generated smoke artifacts after a passing run", () => {
    const removed: string[] = [];

    const status = cleanupGeneratedArtifactsAfterPass({
      appDir,
      status: 0,
      env: {},
      rm: (target) => removed.push(target.toString()),
    });

    expect(status).toBe("cleaned");
    expect(removed).toEqual([
      path.join(appDir, "test-artifacts"),
      path.join(appDir, "test-results"),
      path.join(appDir, "playwright-report"),
    ]);
  });

  test("preserves generated smoke artifacts on failure or explicit keep", () => {
    const removed: string[] = [];

    expect(
      cleanupGeneratedArtifactsAfterPass({
        appDir,
        status: 1,
        env: {},
        rm: (target) => removed.push(target.toString()),
      }),
    ).toBe("preserved_failed_run");
    expect(
      cleanupGeneratedArtifactsAfterPass({
        appDir,
        status: 0,
        env: { DESKTOPLAB_KEEP_TEST_ARTIFACTS: "1" },
        rm: (target) => removed.push(target.toString()),
      }),
    ).toBe("preserved_by_env");
    expect(removed).toEqual([]);
  });
});
