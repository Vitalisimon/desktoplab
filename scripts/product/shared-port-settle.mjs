import { spawnSync } from "node:child_process";

const DEFAULT_PORTS = [1420, 1421];
const PROBE_SOURCE = `
const net = require("node:net");
const server = net.createServer();
server.once("error", () => process.exit(1));
server.listen(Number(process.argv[1]), "127.0.0.1", () => server.close(() => process.exit(0)));
`;

export function waitForSharedProductPorts({
  ports = DEFAULT_PORTS,
  timeoutMs = 10_000,
  intervalMs = 100,
  probe = portIsReleased,
  now = () => Date.now(),
  sleep = sleepSync,
} = {}) {
  const deadline = now() + timeoutMs;
  let blocked = ports.filter((port) => !probe(port));
  while (blocked.length > 0 && now() < deadline) {
    sleep(intervalMs);
    blocked = ports.filter((port) => !probe(port));
  }
  if (blocked.length > 0) {
    throw new Error(`shared product ports were not released: ${blocked.join(", ")}`);
  }
}

function portIsReleased(port) {
  return spawnSync(process.execPath, ["-e", PROBE_SOURCE, String(port)], {
    stdio: "ignore",
    timeout: 2_000,
  }).status === 0;
}

function sleepSync(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
