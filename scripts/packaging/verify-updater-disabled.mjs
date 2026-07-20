#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const config = fs.readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8");
const diagnostics = fs.readFileSync("crates/desktoplab-control-plane/src/router/diagnostics.rs", "utf8");
const install = fs.readFileSync("docs-public/install.md", "utf8");
for (const forbidden of ["\"updater\"", "releases.desktoplab.ai", "PACKAGING_GATE_REQUIRES_REAL_UPDATER_PUBKEY"]) {
  if (config.includes(forbidden)) throw new Error(`bundle still contains updater placeholder: ${forbidden}`);
}
for (const required of ["\"state\":\"disabled\"", "\"canInstall\":false", "In-app updates are disabled for this build"]) {
  if (!diagnostics.includes(required)) throw new Error(`diagnostics missing updater disabled evidence: ${required}`);
}
if (!install.includes("In-app update checks are disabled")) throw new Error("public install guide does not disclose disabled in-app updates");
const report = {
  kind: "desktoplab.updater-disabled-proof",
  schemaVersion: 1,
  status: "passed",
  head: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
  delivery: "disabled",
  hostedManifest: false,
  updaterPublicKey: false,
  installPolicy: "manual-replacement",
};
const reportPath = "dist/release/updater-disabled-proof.json";
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report));
