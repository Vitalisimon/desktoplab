import fs from "node:fs";

const PASS_STATES = ["installState", "launchState", "localApiState", "cleanupState"];

export function buildLinuxHostEvidence({ manifest, smokeLogs, host, commit }) {
  if (manifest?.build?.commitSha !== commit || manifest?.build?.treeState !== "clean") {
    throw new Error("Linux host evidence requires a clean current-head artifact manifest");
  }
  const packages = Object.fromEntries(Object.entries(smokeLogs).map(([format, logPath]) => {
    const result = readSmokeJson(logPath);
    for (const state of PASS_STATES) {
      if (result[state] !== "passed") throw new Error(`${format} smoke did not pass ${state}`);
    }
    if (result.setupState !== "auth_required") throw new Error(`${format} smoke did not preserve the local API auth boundary`);
    const artifact = manifest.entries.find((entry) => entry.relativePath === result.artifact);
    if (!artifact) throw new Error(`${format} smoke artifact is absent from the manifest`);
    if (artifact.signatureState !== "unsigned_dev") throw new Error(`${format} evidence expected an unsigned dev artifact`);
    return [format, { ...result, sha256: artifact.sha256, sizeBytes: artifact.sizeBytes, signatureState: artifact.signatureState }];
  }));
  return {
    kind: "linux-host",
    schemaVersion: 1,
    status: "pass",
    commit,
    platform: "linux-x64",
    publicTrust: false,
    host,
    packages,
  };
}

export function readSmokeJson(logPath) {
  const lines = fs.readFileSync(logPath, "utf8").split(/\r?\n/).filter(Boolean);
  for (const line of lines) {
    try {
      const value = JSON.parse(line);
      if (value?.platform === "linux-x64") return value;
    } catch {}
  }
  throw new Error(`Linux smoke JSON missing from ${logPath}`);
}
