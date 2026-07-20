import fs from "node:fs";

const PASS_STATES = ["installState", "launchState", "localApiState", "cleanupState"];

export function buildWindowsHostEvidence({ manifest, smokeLog, host, commit }) {
  if (manifest?.build?.commitSha !== commit || manifest?.build?.treeState !== "clean") {
    throw new Error("Windows host evidence requires a clean current-head artifact manifest");
  }
  if (manifest.build.signingTrustMode !== "test") {
    throw new Error("Windows development host evidence requires explicit test signing trust");
  }
  if (!manifest.build.runner?.startsWith("physical:windows")) {
    throw new Error("Windows host evidence requires a declared physical Windows runner");
  }

  const smoke = readWindowsSmokeJson(smokeLog);
  for (const state of PASS_STATES) {
    if (smoke[state] !== "passed") throw new Error(`Windows smoke did not pass ${state}`);
  }
  if (smoke.setupState !== "auth_required") {
    throw new Error("Windows smoke did not preserve the local API auth boundary");
  }
  if (smoke.signatureState !== "valid") {
    throw new Error("Windows smoke did not validate Authenticode");
  }

  const artifact = manifest.entries?.find((entry) =>
    entry.fileName === smoke.artifact || normalize(entry.relativePath).endsWith(`/${smoke.artifact}`),
  );
  if (!artifact) throw new Error("Windows smoke artifact is absent from the manifest");
  if (artifact.target !== "windows-x64" || artifact.signatureState !== "signed") {
    throw new Error("Windows host evidence requires a signed artifact manifest entry");
  }

  return {
    kind: "windows-host",
    schemaVersion: 1,
    status: "pass",
    commit,
    platform: "windows-x64",
    publicTrust: false,
    host,
    signing: { trustMode: "test", publicTrust: false },
    artifact: {
      fileName: artifact.fileName,
      sha256: artifact.sha256,
      sizeBytes: artifact.sizeBytes,
      signatureState: artifact.signatureState,
    },
    smoke,
  };
}

export function readWindowsSmokeJson(logPath) {
  const lines = fs.readFileSync(logPath, "utf8").split(/\r?\n/).filter(Boolean);
  for (const line of lines) {
    try {
      const value = JSON.parse(line);
      if (value?.platform === "windows-x64") return value;
    } catch {}
  }
  throw new Error(`Windows smoke JSON missing from ${logPath}`);
}

function normalize(value = "") {
  return value.replaceAll("\\", "/");
}
