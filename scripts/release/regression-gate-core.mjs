import { spawnSync } from "node:child_process";

export function aggregateRun(steps) {
  const required = steps.filter((step) => step.required !== false);
  return {
    status: required.every((step) => step.status === "passed") ? "pass" : "blocked",
    passed: required.filter((step) => step.status === "passed").length,
    blocked: required.filter((step) => step.status !== "passed").length,
  };
}

export function runCommand(step, { cwd = process.cwd(), env = process.env, dryRun = false } = {}) {
  const { env: stepEnv = {}, ...reportedStep } = step;
  const commandText = [step.command, ...step.args].join(" ");
  if (dryRun) {
    return { ...reportedStep, status: "planned", commandText, durationMs: 0, exitCode: null, outputTail: "" };
  }
  const started = Date.now();
  const result = spawnSync(step.command, step.args, {
    cwd,
    env: { ...env, ...stepEnv, CI: stepEnv.CI ?? env.CI ?? "true" },
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    timeout: step.timeoutMs ?? 60 * 60 * 1000,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  const unavailable = result.error?.code === "ENOENT";
  const rejectedOutput = step.rejectOutput === true && output.trim().length > 0;
  return {
    ...reportedStep,
    status: result.status === 0 && !rejectedOutput ? "passed" : unavailable ? "blocked" : "failed",
    commandText,
    durationMs: Date.now() - started,
    exitCode: result.status,
    signal: result.signal ?? null,
    error: result.error?.message ?? null,
    outputTail: output.slice(-8_000),
  };
}

export function appendRun(previous, run) {
  const runs = Array.isArray(previous?.runs) ? previous.runs : [];
  return {
    kind: "desktoplab.safe-signing-regression",
    schemaVersion: 1,
    status: run.status,
    latestRunId: run.runId,
    runs: [...runs, run],
  };
}
