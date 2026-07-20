import { readFileSync } from "node:fs";
import vm from "node:vm";

const fixturePath = process.argv[2];
const policy = JSON.parse(process.env.DESKTOPLAB_PLUGIN_FIXTURE_POLICY ?? "{}");
const events = [];

try {
  const source = readFileSync(fixturePath, "utf8");
  if (Buffer.byteLength(source) > 256 * 1024) throw new Error("fixture_too_large");
  const desktoplab = fixtureSdk(policy, events);
  const context = vm.createContext({ desktoplab }, {
    codeGeneration: { strings: false, wasm: false },
  });
  const script = new vm.Script(source, { filename: "plugin-fixture.js" });
  const value = script.runInContext(context, { timeout: 500 });
  await settleWithTimeout(Promise.resolve(value), 750);
  desktoplab.lifecycle.shutdown();
  write({ status: "completed", reason: null, events });
} catch (error) {
  write({ status: "blocked", reason: classify(error), events });
  process.exitCode = 2;
}

function fixtureSdk(grants, eventLog) {
  const requireGrant = (permission, operation) => {
    if (!grants[permission]) throw new Error(`permission_denied:${permission}`);
    eventLog.push({ kind: operation, source: "fixture_mock" });
  };
  return Object.freeze({
    register: Object.freeze({
      tool(id) { eventLog.push({ kind: "register.tool", id }); },
      hook(id) { eventLog.push({ kind: "register.hook", id }); },
    }),
    network: Object.freeze({ request() { requireGrant("network", "network.mock"); return { status: 200 }; } }),
    vault: Object.freeze({ read() { requireGrant("vault", "vault.mock"); return "fixture-secret-ref"; } }),
    workspace: Object.freeze({ read() { requireGrant("workspace", "workspace.mock"); return "fixture-content"; } }),
    lifecycle: Object.freeze({ shutdown() { eventLog.push({ kind: "lifecycle.shutdown" }); } }),
  });
}

function settleWithTimeout(promise, milliseconds) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error("fixture_timeout")), milliseconds);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

function classify(error) {
  const message = String(error?.message ?? error);
  if (message.includes("permission_denied:")) return message;
  if (message.includes("is not defined") && /process|require|fetch/.test(message)) return "forbidden_global_access";
  if (message.includes("Script execution timed out") || message.includes("fixture_timeout")) return "fixture_timeout";
  if (message.includes("fixture_too_large")) return "fixture_too_large";
  return "fixture_runtime_error";
}

function write(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}
