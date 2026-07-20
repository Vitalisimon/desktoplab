#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { hashArtifact, readEmbeddedBuild } from "./artifact-provenance-core.mjs";

export function verifyMacosCandidateInstall({ candidate, sourceApp, installedApp }) {
  const failures = [];
  if (candidate?.kind !== "desktoplab.release-candidate" || candidate?.schemaVersion !== 1 || candidate?.state !== "payload_built") {
    failures.push("candidate is not an admitted pre-sign payload");
  }
  if (!sourceApp || !existsSync(sourceApp)) failures.push("prepared source app is missing");
  if (!installedApp || !existsSync(installedApp)) failures.push("installed app is missing");
  if (failures.length > 0) return report(candidate, failures);
  const sourceHash = hashArtifact(sourceApp).sha256;
  const installedHash = hashArtifact(installedApp).sha256;
  if (sourceHash !== candidate.payload?.sha256) failures.push("prepared app hash differs from candidate payload");
  if (installedHash !== sourceHash) failures.push("installed app bytes differ from prepared app");
  let sourceBuild;
  let installedBuild;
  try { sourceBuild = readEmbeddedBuild(sourceApp); } catch (error) { failures.push(`prepared app build metadata unavailable: ${error.message}`); }
  try { installedBuild = readEmbeddedBuild(installedApp); } catch (error) { failures.push(`installed app build metadata unavailable: ${error.message}`); }
  if (sourceBuild?.commitSha !== candidate.source?.commit || installedBuild?.commitSha !== candidate.source?.commit) failures.push("installed app commit differs from candidate source");
  if (JSON.stringify(sourceBuild) !== JSON.stringify(installedBuild)) failures.push("installed app build metadata differs from prepared app");
  return report(candidate, failures, { sourceHash, installedHash, sourceBuild, installedBuild });
}

function report(candidate, failures, provenance = {}) {
  return {
    kind: "desktoplab.macos-candidate-install-verification",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    candidateId: candidate?.candidateId ?? null,
    ...provenance,
    failures,
  };
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)) {
  const candidatePath = argument("--candidate");
  const candidate = candidatePath && existsSync(candidatePath) ? JSON.parse(readFileSync(candidatePath, "utf8")) : null;
  const result = verifyMacosCandidateInstall({
    candidate,
    sourceApp: argument("--source-app"),
    installedApp: argument("--installed-app"),
  });
  console.log(JSON.stringify(result, null, 2));
  process.exitCode = result.status === "pass" ? 0 : 1;
}
