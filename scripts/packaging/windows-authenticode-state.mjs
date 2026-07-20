import { spawnSync } from "node:child_process";

export function windowsAuthenticodeState(
  artifactPath,
  { attempts = 8, retryDelayMs = 500, readStatus = readAuthenticodeStatus, wait = sleep } = {},
) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if (readStatus(artifactPath) === "Valid") return "signed";
    if (attempt + 1 < attempts) wait(retryDelayMs);
  }
  return "unsigned_dev";
}

function readAuthenticodeStatus(artifactPath) {
  const nativeVerification = spawnSync(
    "signtool.exe",
    ["verify", "/pa", artifactPath],
    { encoding: "utf8" },
  );
  if (nativeVerification.status === 0) return "Valid";

  const escapedPath = artifactPath.replaceAll("'", "''");
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-Command", `(Get-AuthenticodeSignature -LiteralPath '${escapedPath}').Status`],
    { encoding: "utf8" },
  );
  return result.status === 0 ? result.stdout.trim() : null;
}

function sleep(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
