#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  cargoTestExecutables,
  crossPlatformAgentParityCommands,
  releaseAgentSurfaceFailures,
} from "./cross-platform-agent-parity-core.mjs";

const WINDOWS_TRUST_SETTLE_MS = 5_000;
const rustcWrapper = prepareSignedWindowsRustcWrapper();
let exitCode = rustcWrapper.status;

for (const step of exitCode === 0 ? crossPlatformAgentParityCommands() : []) {
  console.log(`\n> ${step.command} ${step.args.join(" ")}`);
  const result = signedWindowsTestCertificate() && step.kind === "test"
    ? runSignedWindowsStep(step)
    : run(step.command, step.args);
  if (result.error) {
    console.error(`Failed to start ${step.command}: ${result.error.message}`);
    exitCode = 1;
    break;
  }
  if (result.status !== 0) {
    exitCode = result.status ?? 1;
    break;
  }
}

if (exitCode === 0) {
  const binary = resolve(
    "target",
    "release",
    process.platform === "win32" ? "desktoplab-local-api.exe" : "desktoplab-local-api",
  );
  const failures = existsSync(binary)
    ? releaseAgentSurfaceFailures(readFileSync(binary))
    : [`release agent binary is missing: ${binary}`];
  if (failures.length > 0) {
    for (const failure of failures) console.error(failure);
    exitCode = 1;
  }
}

if (rustcWrapper.path && existsSync(rustcWrapper.path)) unlinkSync(rustcWrapper.path);
if (exitCode === 0) {
  console.log(`\nCross-platform agent parity gate passed on ${process.platform}/${process.arch}`);
}
process.exitCode = exitCode;

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: process.cwd(),
    env: process.env,
    stdio: "inherit",
    shell: false,
    ...options,
  });
}

function runSignedWindowsStep(step) {
  const buildArgs = [step.args[0], "--no-run", "--message-format=json", ...step.args.slice(1)];
  const build = run(step.command, buildArgs, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (build.stderr) process.stderr.write(build.stderr);
  if (build.error || build.status !== 0) return build;
  const executables = cargoTestExecutables(build.stdout);
  if (executables.length === 0) {
    return { status: 1, error: new Error("Cargo produced no test executables") };
  }
  for (const executable of executables) {
    const signed = signWindowsTestExecutable(executable);
    if (signed.error || signed.status !== 0) return signed;
    waitForWindowsTrustPropagation();
    const tested = run(executable, []);
    if (tested.error || tested.status !== 0) return tested;
  }
  return { status: 0, error: null };
}

function waitForWindowsTrustPropagation() {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, WINDOWS_TRUST_SETTLE_MS);
}

function signedWindowsTestCertificate() {
  const thumbprint = process.env.DESKTOPLAB_WINDOWS_TEST_CERT_THUMBPRINT?.trim() ?? "";
  return process.platform === "win32" && /^[a-f0-9]{40}$/i.test(thumbprint)
    ? thumbprint
    : null;
}

function prepareSignedWindowsRustcWrapper() {
  if (!signedWindowsTestCertificate()) return { status: 0, path: null };
  const outputPath = join(tmpdir(), `desktoplab-rustc-sign-wrapper-${process.pid}.exe`);
  const bootstrap = resolve("scripts/packaging/windows-rustc-signing-bootstrap.ps1");
  const result = spawnSync(
    "pwsh.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", bootstrap, "-OutputPath", outputPath],
    { cwd: process.cwd(), env: process.env, stdio: "inherit", shell: false },
  );
  if (result.error || result.status !== 0) return { status: result.status ?? 1, path: outputPath };
  process.env.RUSTC_WRAPPER = outputPath;
  return { status: 0, path: outputPath };
}

function signWindowsTestExecutable(executable) {
  const signingScript = resolve("scripts/packaging/windows-sign.ps1");
  return spawnSync(
    "pwsh.exe",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      signingScript,
      "-ArtifactPath",
      executable,
      "-TrustMode",
      "Test",
    ],
    { cwd: process.cwd(), env: process.env, stdio: "inherit", shell: false },
  );
}
