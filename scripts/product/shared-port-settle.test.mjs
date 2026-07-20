import assert from "node:assert/strict";
import test from "node:test";

import { waitForSharedProductPorts } from "./shared-port-settle.mjs";

test("waits until every shared product port is released", () => {
  let time = 0;
  let attempts = 0;
  assert.doesNotThrow(() => waitForSharedProductPorts({
    ports: [1420, 1421],
    timeoutMs: 500,
    intervalMs: 100,
    now: () => time,
    sleep: (milliseconds) => { time += milliseconds; },
    probe: () => ++attempts > 3,
  }));
  assert.equal(time, 200);
});

test("fails with the exact ports that remain occupied", () => {
  let time = 0;
  assert.throws(() => waitForSharedProductPorts({
    ports: [1420, 1421],
    timeoutMs: 200,
    intervalMs: 100,
    now: () => time,
    sleep: (milliseconds) => { time += milliseconds; },
    probe: (port) => port === 1420,
  }), /shared product ports were not released: 1421/);
});
