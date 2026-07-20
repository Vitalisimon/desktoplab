#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import process from "node:process";
import { assertNoConfiguredEntitlements } from "./macos-entitlements-contract.mjs";

const entitlementPath = process.argv[2];
if (!entitlementPath || !fs.existsSync(entitlementPath)) {
  throw new Error(`missing macOS entitlements file: ${entitlementPath ?? "<unspecified>"}`);
}

const configured = JSON.parse(execFileSync(
  "plutil",
  ["-convert", "json", "-o", "-", entitlementPath],
  { encoding: "utf8" },
));
assertNoConfiguredEntitlements(configured);

console.log("macOS entitlement contract passed: no entitlements will be embedded.");
