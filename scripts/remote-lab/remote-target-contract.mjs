import { spawnSync } from "node:child_process";

const KINDS = new Set(["local_owned", "static_ssh", "future_leased"]);
const PLATFORMS = new Set(["macos", "linux", "windows"]);
const TRUST = new Set(["local_owner", "trusted_physical", "external_lease"]);

export function validateTarget(target) {
  const failures = [];
  if (!/^[a-z0-9][a-z0-9._-]{1,63}$/.test(target?.id ?? "")) failures.push("invalid_id");
  if (!KINDS.has(target?.kind)) failures.push("invalid_kind");
  if (!PLATFORMS.has(target?.platform)) failures.push("invalid_platform");
  if (!/^(x64|arm64)$/.test(target?.architecture ?? "")) failures.push("invalid_architecture");
  if (!TRUST.has(target?.trustLevel)) failures.push("invalid_trust_level");
  if (!Array.isArray(target?.capabilities) || target.capabilities.length === 0) failures.push("missing_capabilities");
  if (target?.kind === "static_ssh" && (!target.endpointRef || !target.credentialRef)) failures.push("missing_ssh_references");
  for (const forbidden of ["host", "username", "password", "privateKey", "token"]) {
    if (Object.hasOwn(target ?? {}, forbidden)) failures.push(`embedded_credential_or_endpoint:${forbidden}`);
  }
  return { valid: failures.length === 0, failures };
}

export class RemoteTargetController {
  constructor(targets, transport) {
    for (const target of targets) {
      const validation = validateTarget(target);
      if (!validation.valid) throw new Error(`${target?.id ?? "target"}:${validation.failures.join(",")}`);
    }
    this.targets = new Map(targets.map((target) => [target.id, structuredClone(target)]));
    this.transport = transport;
    this.leases = new Map();
  }

  probe(targetId) {
    const target = this.#target(targetId);
    const result = this.transport.probe(target);
    return {
      targetId,
      state: result.status === 0 ? "available" : "offline",
      reason: result.status === 0 ? null : result.reason ?? "probe_failed",
      fingerprint: result.fingerprint ?? null,
    };
  }

  claim(targetId, runId, ownerId) {
    this.#target(targetId);
    const existing = this.leases.get(targetId);
    if (existing && existing.runId !== runId) throw new Error("target_already_claimed");
    const lease = { targetId, runId, ownerId, state: "claimed" };
    this.leases.set(targetId, lease);
    return { ...lease };
  }

  prepare(targetId, runId, ownerId) {
    return this.#transition(targetId, runId, ownerId, "prepared");
  }

  run(targetId, runId, ownerId, command) {
    const lease = this.#lease(targetId, runId, ownerId);
    if (lease.state !== "prepared") throw new Error("target_not_prepared");
    lease.state = "running";
    const result = this.transport.run(this.#target(targetId), command);
    lease.state = result.status === 0 ? "completed" : "failed";
    return { lease: { ...lease }, result };
  }

  cancel(targetId, runId, ownerId) {
    const lease = this.#lease(targetId, runId, ownerId);
    this.transport.cancel?.(this.#target(targetId), runId);
    lease.state = "cancelled";
    return { ...lease };
  }

  collect(targetId, runId, ownerId) {
    const lease = this.#lease(targetId, runId, ownerId);
    if (!["completed", "failed", "cancelled"].includes(lease.state)) throw new Error("run_not_terminal");
    return this.transport.collect?.(this.#target(targetId), runId) ?? { artifacts: [] };
  }

  release(targetId, runId, ownerId) {
    const lease = this.#lease(targetId, runId, ownerId);
    this.leases.delete(targetId);
    return { ...lease, state: "released" };
  }

  #transition(targetId, runId, ownerId, state) {
    const lease = this.#lease(targetId, runId, ownerId);
    lease.state = state;
    return { ...lease };
  }

  #lease(targetId, runId, ownerId) {
    const lease = this.leases.get(targetId);
    if (!lease || lease.runId !== runId || lease.ownerId !== ownerId) throw new Error("target_lease_owner_mismatch");
    return lease;
  }

  #target(targetId) {
    const target = this.targets.get(targetId);
    if (!target) throw new Error("target_not_found");
    return target;
  }
}

export class SystemTargetTransport {
  constructor(environment = process.env) {
    this.environment = environment;
  }

  probe(target) {
    if (target.kind === "future_leased") return { status: 2, reason: "lease_provider_not_configured" };
    const command = target.platform === "windows"
      ? "Write-Output desktoplab-probe; Write-Output $env:OS; Write-Output $env:PROCESSOR_ARCHITECTURE"
      : "printf 'desktoplab-probe\\n'; uname -s; uname -m";
    const result = this.run(target, command);
    return {
      status: result.status,
      reason: result.status === 0 ? null : result.reason ?? classifySshFailure(result.stderr),
      fingerprint: result.status === 0 ? result.stdout.trim().split(/\r?\n/).slice(-3) : null,
    };
  }

  run(target, command) {
    const invocation = target.kind === "local_owned"
      ? localInvocation(target, command)
      : sshInvocation(target, command, this.environment);
    if (invocation.error) return invocation;
    const result = spawnSync(invocation.command, invocation.args, {
      encoding: "utf8",
      timeout: target.timeoutMs ?? 15_000,
      maxBuffer: 4 * 1024 * 1024,
      shell: false,
    });
    return {
      status: result.status ?? 1,
      stdout: bounded(result.stdout),
      stderr: bounded(result.stderr),
      reason: result.error?.code === "ETIMEDOUT" ? "probe_timeout" : result.error?.message ?? null,
    };
  }

  upload(target, localPath, remotePath) {
    if (!/^[a-zA-Z0-9._\/-]+$/.test(remotePath) || target.kind !== "static_ssh") {
      return { status: 2, reason: "invalid_upload_target", stdout: "", stderr: "" };
    }
    const configuration = staticSshConfiguration(target, this.environment);
    if (configuration.error) return configuration;
    const result = spawnSync("scp", [
      "-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "-i", configuration.identity,
      localPath, `${configuration.endpoint}:${remotePath}`,
    ], { encoding: "utf8", timeout: target.timeoutMs ?? 30_000, maxBuffer: 4 * 1024 * 1024, shell: false });
    const stderr = bounded(result.stderr);
    return { status: result.status ?? 1, stdout: bounded(result.stdout), stderr, reason: result.error?.message ?? (result.status === 0 ? null : classifySshFailure(stderr)) };
  }
}

function sshInvocation(target, command, environment) {
  const configuration = staticSshConfiguration(target, environment);
  if (configuration.error) return configuration;
  const remoteCommand = target.platform === "windows"
    ? `powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand ${Buffer.from(command, "utf16le").toString("base64")}`
    : command;
  return {
    command: "ssh",
    args: ["-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "-i", configuration.identity, configuration.endpoint, remoteCommand],
  };
}

function staticSshConfiguration(target, environment) {
  const endpoint = environment[target.endpointRef]?.trim();
  const identity = environment[target.credentialRef]?.replace(/^~(?=\/)/, environment.HOME ?? "")?.trim();
  return endpoint && identity ? { endpoint, identity } : { status: 2, reason: "target_configuration_missing", stdout: "", stderr: "", error: true };
}

function localInvocation(target, command) {
  return target.platform === "windows"
    ? { command: "powershell.exe", args: ["-NoProfile", "-NonInteractive", "-Command", command] }
    : { command: "/bin/sh", args: ["-lc", command] };
}

function bounded(value = "") {
  return String(value).slice(0, 64 * 1024);
}

function classifySshFailure(stderr = "") {
  const value = stderr.toLowerCase();
  if (value.includes("could not resolve hostname")) return "endpoint_dns_unavailable";
  if (value.includes("connection timed out") || value.includes("operation timed out")) return "target_unreachable_or_sleeping";
  if (value.includes("connection refused")) return "ssh_service_unavailable";
  if (value.includes("permission denied")) return "ssh_authentication_failed";
  if (value.includes("host key verification failed")) return "ssh_host_key_untrusted";
  return "probe_failed";
}
