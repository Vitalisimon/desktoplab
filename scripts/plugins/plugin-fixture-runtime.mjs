import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const worker = fileURLToPath(new URL("./plugin-fixture-worker.mjs", import.meta.url));

export function runPluginFixture({
  fixturePath,
  staticReport,
  execute = false,
  declaredPermissions = [],
  grantedPermissions = [],
  timeoutMs = 2_000,
}) {
  if (!execute) throw new Error("plugin_fixture_execution_requires_explicit_flag");
  if (staticReport?.compatible !== true || staticReport.executedPluginCode !== false) {
    throw new Error("compatible_static_inspection_required");
  }
  if (!Number.isInteger(timeoutMs) || timeoutMs < 100 || timeoutMs > 5_000) {
    throw new Error("invalid_plugin_fixture_timeout");
  }
  const policy = permissionPolicy(declaredPermissions, grantedPermissions);
  const result = spawnSync(process.execPath, [worker, fixturePath], {
    encoding: "utf8",
    timeout: timeoutMs,
    maxBuffer: 256 * 1024,
    shell: false,
    env: minimalEnvironment(policy),
  });
  if (result.error?.code === "ETIMEDOUT") {
    return outcome("blocked", "fixture_timeout", policy, []);
  }
  if (result.status !== 0) {
    const parsed = parseWorkerOutput(result.stdout);
    return outcome("blocked", parsed?.reason ?? "fixture_process_failed", policy, parsed?.events ?? []);
  }
  const parsed = parseWorkerOutput(result.stdout);
  if (!parsed || parsed.status !== "completed") {
    return outcome("blocked", "fixture_output_invalid", policy, []);
  }
  return { ...parsed, isolationProfile: isolationProfile(policy) };
}

function permissionPolicy(declared, granted) {
  const declaredSet = new Set(declared);
  const grantedSet = new Set(granted);
  return Object.fromEntries(["network", "vault", "workspace"].map((permission) => [
    permission,
    declaredSet.has(permission) && grantedSet.has(permission),
  ]));
}

function minimalEnvironment(policy) {
  return {
    PATH: process.env.PATH ?? "",
    SYSTEMROOT: process.env.SYSTEMROOT ?? "",
    WINDIR: process.env.WINDIR ?? "",
    TMPDIR: process.env.TMPDIR ?? "",
    TEMP: process.env.TEMP ?? "",
    DESKTOPLAB_PLUGIN_FIXTURE_POLICY: JSON.stringify(policy),
  };
}

function parseWorkerOutput(stdout) {
  const line = stdout.trim().split(/\r?\n/).at(-1);
  try {
    return JSON.parse(line);
  } catch {
    return null;
  }
}

function isolationProfile(policy) {
  return {
    kind: "bounded_node_vm_fixture",
    productionSandbox: false,
    processEnvironment: "allowlisted",
    codeGeneration: false,
    realNetwork: false,
    realVault: false,
    realWorkspace: false,
    grantedFixtureMocks: Object.entries(policy).filter(([, value]) => value).map(([key]) => key),
  };
}

function outcome(status, reason, policy, events) {
  return { status, reason, events, isolationProfile: isolationProfile(policy) };
}
