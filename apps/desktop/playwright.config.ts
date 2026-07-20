import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(appDir, "../..");
const packagedLocalApiSmoke = process.argv.some((arg: string) =>
  [
    "macos-packaged-launch.spec.ts",
    "packaged-local-api.spec.ts",
    "packaged-product-launch.spec.ts",
    "packaged-shell.spec.ts",
  ].some((file) => arg.includes(file)),
);
const packagedSmokeToken = "desktoplab-packaged-smoke-token";
const productLocalApiSmoke = process.argv.some((arg: string) => arg.includes("tests/product"));
const frontierSetupSmoke = process.argv.some((arg: string) => arg.includes("frontier-local-setup.spec.ts"));
const localApiCommand = packagedLocalApiSmoke
  ? `cargo run -p desktoplab-control-plane --bin desktoplab-local-api -- --auth-token ${packagedSmokeToken}`
  : frontierSetupSmoke
    ? "DESKTOPLAB_AGENT_BACKEND_MODE=fail DESKTOPLAB_TEST_CONTROLS=1 DESKTOPLAB_FRONTIER_SETUP_TEST_PROFILE=dgx_station_class DESKTOPLAB_FRONTIER_RUNTIME_TEST_ENDPOINT=http://127.0.0.1:18000 cargo run -p desktoplab-control-plane --bin desktoplab-local-api -- --unsafe-no-auth"
    : "DESKTOPLAB_AGENT_BACKEND_MODE=fail DESKTOPLAB_TEST_CONTROLS=1 cargo run -p desktoplab-control-plane --bin desktoplab-local-api -- --unsafe-no-auth";
const webServer = [
  {
    command: packagedLocalApiSmoke
      ? `VITE_DESKTOPLAB_ALLOW_BROWSER_DEV_FALLBACK=1 VITE_DESKTOPLAB_API_AUTH_TOKEN=${packagedSmokeToken} npm run dev -- --port 1420`
      : "VITE_DESKTOPLAB_ALLOW_BROWSER_DEV_FALLBACK=1 npm run dev -- --port 1420",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: false,
    timeout: 30_000,
  },
  {
    command: localApiCommand,
    cwd: repoRoot,
    url: "http://127.0.0.1:1421/health",
    reuseExistingServer: false,
    timeout: 30_000,
  },
  ...(frontierSetupSmoke
    ? [{ command: "node tests/fixtures/openai-compatible-runtime.mjs", url: "http://127.0.0.1:18000/v1/models", reuseExistingServer: false, timeout: 10_000 }]
    : []),
];

export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  workers: productLocalApiSmoke ? 1 : undefined,
  expect: {
    timeout: 5_000,
  },
  use: {
    baseURL: "http://127.0.0.1:1420",
    trace: "on-first-retry",
  },
  webServer,
  projects: [
    {
      name: "desktop",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 820 } },
    },
    {
      name: "narrow",
      use: { ...devices["Desktop Chrome"], viewport: { width: 980, height: 720 } },
    },
  ],
});
