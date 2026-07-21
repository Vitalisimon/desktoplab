import { fileURLToPath } from "node:url";

export const installedAgentUiWaitModulePath = fileURLToPath(import.meta.url);

export function latestTerminalTurn(session, afterUnixMs) {
  const trace = [...(session?.trace ?? [])].reverse().find((event) =>
    ["completed", "failed", "cancelled"].includes(event.kind)
      && event.recordedAtUnixMs >= afterUnixMs,
  );
  if (!trace) return null;
  const lifecycle = [...(session?.events ?? [])].reverse().find((event) => event.kind === trace.kind);
  return { kind: trace.kind, reason: lifecycle?.reason ?? trace.kind };
}

export async function waitForActiveUi(ui, probe, timeoutMs, label, dependencies = {}) {
  const wait = dependencies.wait ?? waitFor;
  const now = dependencies.now ?? Date.now;
  const reactivateAfterMs = dependencies.reactivateAfterMs ?? 5_000;
  let lastActivationAt = Number.NEGATIVE_INFINITY;
  return wait(() => {
    const current = now();
    if (current - lastActivationAt >= reactivateAfterMs) {
      ui.activate();
      lastActivationAt = current;
    }
    return probe();
  }, timeoutMs, label);
}

async function waitFor(probe, timeoutMs, label) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try { const value = await probe(); if (value) return value; } catch {}
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`timed out waiting for ${label}`);
}
