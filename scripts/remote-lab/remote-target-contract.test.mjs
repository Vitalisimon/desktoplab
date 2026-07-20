import assert from "node:assert/strict";
import test from "node:test";

import { RemoteTargetController, validateTarget } from "./remote-target-contract.mjs";
import { localTargetInventory } from "./remote-targets.mjs";

const transport = {
  probe: (target) => target.id === "offline" ? { status: 1, reason: "timeout" } : { status: 0, fingerprint: [target.platform, target.architecture] },
  run: (_target, command) => ({ status: command === "fail" ? 1 : 0, stdout: "ok", stderr: "" }),
  collect: (_target, runId) => ({ artifacts: [`${runId}.json`] }),
};

test("physical target descriptors contain references but no endpoint or credential material", () => {
  const targets = localTargetInventory();
  assert.ok(targets.every((target) => validateTarget(target).valid));
  for (const target of targets) {
    const serialized = JSON.stringify(target);
    assert.doesNotMatch(serialized, /192\.168\.|100\.109\.|BEGIN.*PRIVATE KEY/i);
  }
});

test("claim prepare run collect release enforces owner identity", () => {
  const target = { id: "lab", kind: "local_owned", platform: "linux", architecture: "x64", shell: "posix", capabilities: ["agent"], trustLevel: "local_owner" };
  const controller = new RemoteTargetController([target], transport);
  controller.claim("lab", "run-1", "owner-a");
  assert.throws(() => controller.release("lab", "run-1", "owner-b"), /owner_mismatch/);
  controller.prepare("lab", "run-1", "owner-a");
  const executed = controller.run("lab", "run-1", "owner-a", "test");
  assert.equal(executed.lease.state, "completed");
  assert.deepEqual(controller.collect("lab", "run-1", "owner-a").artifacts, ["run-1.json"]);
  assert.equal(controller.release("lab", "run-1", "owner-a").state, "released");
});

test("offline and future leased targets return actionable availability", () => {
  const offline = { id: "offline", kind: "static_ssh", platform: "windows", architecture: "x64", shell: "powershell", capabilities: ["agent"], trustLevel: "trusted_physical", endpointRef: "HOST", credentialRef: "KEY" };
  const controller = new RemoteTargetController([offline], transport);
  assert.deepEqual(controller.probe("offline"), {
    targetId: "offline",
    state: "offline",
    reason: "timeout",
    fingerprint: null,
  });
});

test("system transport source classifies common SSH failures without returning credentials", async () => {
  const { readFile } = await import("node:fs/promises");
  const source = await readFile(new URL("./remote-target-contract.mjs", import.meta.url), "utf8");
  for (const reason of [
    "endpoint_dns_unavailable",
    "target_unreachable_or_sleeping",
    "ssh_service_unavailable",
    "ssh_authentication_failed",
    "ssh_host_key_untrusted",
  ]) assert.match(source, new RegExp(reason));
  assert.doesNotMatch(source, /console\.(log|error).*identity/);
  assert.match(source, /EncodedCommand/);
  assert.match(source, /utf16le/);
});

test("remote target contract stays bounded", async () => {
  const { readFile } = await import("node:fs/promises");
  const source = await readFile(new URL("./remote-target-contract.mjs", import.meta.url), "utf8");
  const logical = source.split("\n").filter((line) => line.trim() && !line.trim().startsWith("//")).length;
  assert.ok(logical <= 260, `remote target contract has ${logical} logical lines`);
});
