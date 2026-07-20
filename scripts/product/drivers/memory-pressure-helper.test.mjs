import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { observeProcessMemory, parseMemorySizeKb } from "./process-memory-observation.mjs";

const helper = fileURLToPath(new URL("./memory-pressure-helper.mjs", import.meta.url));

test("memory pressure helper keeps non-compressible pages resident", { skip: process.platform === "win32" }, async () => {
  const requestedMb = 64;
  const child = spawn(process.execPath, [helper, String(requestedMb)], { stdio: ["ignore", "pipe", "pipe"] });
  try {
    const ready = await waitFor(() => child.stdout.read()?.toString().includes(`ready ${requestedMb * 1024 * 1024}`), 15_000);
    assert.equal(ready, true);
    const memory = await waitFor(() => {
      const observed = observeProcessMemory(child.pid);
      return observed?.observedMemoryKb >= requestedMb * 900 ? observed : null;
    }, 15_000);
    assert.ok(memory.observedMemoryKb >= requestedMb * 900, `observed memory was ${memory.observedMemoryKb} KiB`);
    assert.equal(memory.measurement, process.platform === "darwin" ? "physical_footprint" : "rss");
  } finally {
    child.kill("SIGTERM");
  }
});

test("memory size parser normalizes vmmap units to KiB", () => {
  assert.equal(parseMemorySizeKb("780.9M"), 799642);
  assert.equal(parseMemorySizeKb("1.5G"), 1572864);
  assert.equal(parseMemorySizeKb("invalid"), 0);
});

async function waitFor(probe, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = probe();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return null;
}
