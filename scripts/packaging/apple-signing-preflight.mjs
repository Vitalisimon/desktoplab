#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assessAppleSigningPreflight, parseDeveloperIdApplications } from "./apple-signing-preflight-core.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const identityResult = run("security", ["find-identity", "-p", "codesigning", "-v"]);
const identities = parseDeveloperIdApplications(identityResult.output);
const selectedIdentity = process.env.DESKTOPLAB_MACOS_SIGNING_IDENTITY ?? "";
const selectedIdentityMatches = identities.some((identity) =>
  identity.fingerprint === selectedIdentity.toUpperCase() || identity.rawIdentity.includes(selectedIdentity),
);
const notarytool = run("xcrun", ["notarytool", "--version"]);
const profile = process.env.APPLE_KEYCHAIN_PROFILE ?? "";
const notaryValidation = profile
  ? run("xcrun", ["notarytool", "history", "--keychain-profile", profile])
  : { status: null, output: "" };
const appleService = run("curl", ["--head", "--silent", "--show-error", "--max-time", "15", "https://appstoreconnect.apple.com"]);
const timestampService = run("curl", ["--head", "--silent", "--show-error", "--max-time", "15", "http://timestamp.apple.com/ts01"]);
const secretScan = run("node", ["scripts/security/scan-tracked-secrets.mjs"]);
const report = assessAppleSigningPreflight({
  identities,
  selectedIdentityConfigured: selectedIdentity.length > 0,
  selectedIdentityMatches,
  notarytoolAvailable: notarytool.status === 0,
  notaryProfileConfigured: profile.length > 0,
  notaryProfileValidated: notaryValidation.status === 0,
  appleServiceReachable: appleService.status === 0,
  timestampServiceReachable: timestampService.status === 0,
  trackedSecretScanPassed: secretScan.status === 0,
});
const output = resolve(root, "dist/release/apple-signing-preflight.json");
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify({ ...report, generatedAt: new Date().toISOString() }, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
if (report.status !== "pass") process.exitCode = 2;

function run(program, args) {
  const result = spawnSync(program, args, { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
  return { status: result.status, output: `${result.stdout ?? ""}${result.stderr ?? ""}` };
}
