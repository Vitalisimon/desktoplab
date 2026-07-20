#!/usr/bin/env node
import crypto from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { assertNoConfiguredEntitlements } from "./macos-entitlements-contract.mjs";

const appPath = argument("--app") ?? "/Applications/DesktopLab.app";
const reportPath = argument("--report") ?? "dist/release/macos-entitlement-review.json";
const entitlementPath = "apps/desktop/src-tauri/entitlements/macos.plist";
const config = JSON.parse(fs.readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8"));
const configured = plist(entitlementPath);
assertNoConfiguredEntitlements(configured);
expect(config.bundle?.macOS?.hardenedRuntime === true, "hardened runtime must remain enabled");
expect(!("entitlements" in (config.bundle?.macOS ?? {})), "Tauri must not embed an empty entitlement blob");

let signed = {};
if (process.platform === "darwin") {
  const result = spawnSync("codesign", ["-dvvv", "--entitlements", ":-", appPath], { encoding: "utf8" });
  expect(result.status === 0, `${result.stdout ?? ""}${result.stderr ?? ""}`.trim());
  const details = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  expect(!details.includes("invalid entitlements blob"), "signed app contains an invalid entitlement blob");
  const xmlStart = details.indexOf("<?xml");
  if (xmlStart !== -1) {
    const embeddedPath = path.join(process.env.TMPDIR ?? "/tmp", `desktoplab-entitlements-${process.pid}.plist`);
    fs.writeFileSync(embeddedPath, details.slice(xmlStart));
    try {
      expect(Object.keys(plist(embeddedPath)).length === 0, "signed app contains an unreviewed entitlement");
    } finally {
      fs.rmSync(embeddedPath, { force: true });
    }
  }
  expect(/flags=.*runtime/.test(details), "signed app does not have hardened runtime enabled");
  signed = { appPath, hardenedRuntime: true, embeddedEntitlements: [], entitlementBlob: "absent" };
}

const report = {
  kind: "desktoplab.macos-entitlement-review",
  schemaVersion: 1,
  status: "passed",
  head: git(["rev-parse", "HEAD"]),
  entitlementSha256: crypto.createHash("sha256").update(fs.readFileSync(entitlementPath)).digest("hex"),
  appSandbox: { enabled: false, reason: "arbitrary user-selected repositories, approved terminal processes and local runtime management" },
  capabilities: [
    { id: "filesystem", mechanism: "user-selected repository plus DesktopLab policy", entitlement: null },
    { id: "network", mechanism: "loopback local API, local runtimes and approved provider egress", entitlement: null },
    { id: "subprocess", mechanism: "approved terminal and runtime executors", entitlement: null },
    { id: "keychain", mechanism: "macOS security CLI generic passwords", entitlement: null },
    { id: "updater", mechanism: "disabled until signed channel exists", entitlement: null },
  ],
  signed,
};
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report));

function plist(file) {
  return JSON.parse(execFileSync("plutil", ["-convert", "json", "-o", "-", file], { encoding: "utf8" }));
}
function git(values) {
  return execFileSync("git", values, { encoding: "utf8" }).trim();
}
function argument(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? null : process.argv[index + 1];
}
function expect(condition, message) {
  if (!condition) throw new Error(message);
}
