import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPluginFixture } from "./plugin-fixture-runtime.mjs";

const inspected = { compatible: true, executedPluginCode: false };

test("runtime execution requires explicit consent and compatible static evidence", () => {
  const fixturePath = fixture("desktoplab.register.tool('tool.example')");
  assert.throws(() => runPluginFixture({ fixturePath, staticReport: inspected }), /explicit_flag/);
  assert.throws(() => runPluginFixture({ fixturePath, staticReport: { compatible: false }, execute: true }), /static_inspection/);
});

test("bounded fixture registers public SDK surfaces and shuts down", () => {
  const fixturePath = fixture("desktoplab.register.tool('tool.example'); desktoplab.register.hook('shutdown')");
  const result = runPluginFixture({ fixturePath, staticReport: inspected, execute: true });
  assert.equal(result.status, "completed");
  assert.deepEqual(result.events.map(({ kind }) => kind), [
    "register.tool", "register.hook", "lifecycle.shutdown",
  ]);
  assert.equal(result.isolationProfile.productionSandbox, false);
  assert.equal(result.isolationProfile.realNetwork, false);
});

test("forbidden globals and undeclared capabilities are blocked", () => {
  for (const source of ["process.env", "require('node:fs')", "fetch('https://example.test')"]) {
    const result = runPluginFixture({ fixturePath: fixture(source), staticReport: inspected, execute: true });
    assert.equal(result.status, "blocked");
    assert.equal(result.reason, "forbidden_global_access");
  }
  const workspace = runPluginFixture({
    fixturePath: fixture("desktoplab.workspace.read()"),
    staticReport: inspected,
    execute: true,
    declaredPermissions: ["workspace"],
  });
  assert.equal(workspace.reason, "permission_denied:workspace");
});

test("declared and granted capabilities expose fixture mocks only", () => {
  const result = runPluginFixture({
    fixturePath: fixture("desktoplab.workspace.read(); desktoplab.vault.read(); desktoplab.network.request()"),
    staticReport: inspected,
    execute: true,
    declaredPermissions: ["workspace", "vault", "network"],
    grantedPermissions: ["workspace", "vault", "network"],
  });
  assert.equal(result.status, "completed");
  assert.deepEqual(result.isolationProfile.grantedFixtureMocks, ["network", "vault", "workspace"]);
  assert.ok(result.events.every((event) => event.kind.endsWith(".mock") || event.kind === "lifecycle.shutdown"));
});

test("runtime harness sources stay reviewable", () => {
  const runtime = new URL("./plugin-fixture-runtime.mjs", import.meta.url);
  const worker = new URL("./plugin-fixture-worker.mjs", import.meta.url);
  assert.ok(readLines(runtime) <= 150);
  assert.ok(readLines(worker) <= 150);
});

function fixture(source) {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-plugin-fixture-"));
  const path = join(root, "plugin.js");
  writeFileSync(path, source);
  return path;
}

function readLines(url) {
  return readFileSync(url, "utf8").split(/\r?\n/).length;
}
