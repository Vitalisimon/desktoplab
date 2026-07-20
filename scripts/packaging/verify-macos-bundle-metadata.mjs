#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { hashArtifact, readEmbeddedBuild } from "./artifact-provenance-core.mjs";
import { assertSignatureMode, signatureState } from "./macos-signature-policy.mjs";

const args = parseArgs(process.argv.slice(2));
const appPath = args.app ?? "/Applications/DesktopLab.app";
const mode = args.mode ?? (process.env.DESKTOPLAB_PUBLIC_RELEASE === "1" ? "release" : "dev");
if (process.platform !== "darwin") {
  console.log("macOS bundle integrity guard skipped: non-macOS host");
  process.exit(0);
}
verifyBundle(appPath, mode);
if (args.dmg) verifyDmg(args.dmg, appPath, mode);

function verifyBundle(bundlePath, verificationMode) {
  const infoPlist = join(bundlePath, "Contents", "Info.plist");
  expect(existsSync(infoPlist), `missing Info.plist at ${infoPlist}`);
  strictCodesign(bundlePath);
  const inspected = spawnSync("codesign", ["-dvvv", "--entitlements", ":-", bundlePath], { encoding: "utf8" });
  expect(inspected.status === 0, `${inspected.stdout ?? ""}${inspected.stderr ?? ""}`.trim());
  const details = `${inspected.stdout ?? ""}${inspected.stderr ?? ""}`;
  const stapled = spawnSync("xcrun", ["stapler", "validate", bundlePath], { encoding: "utf8" }).status === 0;
  const state = signatureState(details, stapled);
  assertSignatureMode(state, verificationMode);
  const plist = JSON.parse(command("plutil", ["-convert", "json", "-o", "-", infoPlist]));
  verifyPlist(plist);
  const metadata = readEmbeddedBuild(bundlePath);
  const head = command("git", ["rev-parse", "HEAD"]).trim();
  expect(metadata.commitSha === head, `bundle commit ${metadata.commitSha} differs from HEAD ${head}`);
  expect(existsSync(join(bundlePath, "Contents", "Resources", "icon.icns")), "bundle icon resource is missing");
  const binaries = machOBinaries(bundlePath);
  expect(binaries.length > 0, "bundle contains no Mach-O executable");
  for (const binary of binaries) {
    strictCodesign(binary, false);
    const architectures = command("lipo", ["-archs", binary]).trim().split(/\s+/);
    expect(architectures.includes(args.architecture ?? process.arch), `unexpected architecture for ${binary}: ${architectures.join(",")}`);
  }
  console.log(JSON.stringify({ appPath: bundlePath, mode: verificationMode, signatureState: state, architecture: args.architecture ?? process.arch, strictIntegrity: "passed" }));
}

function verifyDmg(dmgPath, candidateApp, verificationMode) {
  expect(existsSync(dmgPath), `missing DMG at ${dmgPath}`);
  const mount = mkdtempSync(join(tmpdir(), "desktoplab-dmg-"));
  try {
    command("hdiutil", ["attach", dmgPath, "-readonly", "-nobrowse", "-mountpoint", mount]);
    const mountedApp = join(mount, "DesktopLab.app");
    expect(existsSync(mountedApp), "DMG does not contain DesktopLab.app");
    verifyBundle(mountedApp, verificationMode);
    expect(hashArtifact(mountedApp).sha256 === hashArtifact(candidateApp).sha256, "DMG app differs from the verified candidate app");
  } finally {
    spawnSync("hdiutil", ["detach", mount, "-force"], { encoding: "utf8" });
    rmSync(mount, { recursive: true, force: true });
  }
}

function strictCodesign(target, deep = true) {
  const values = ["--verify", ...(deep ? ["--deep"] : []), "--strict", "--verbose=4", target];
  const result = spawnSync("codesign", values, { encoding: "utf8" });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  expect(result.status === 0, output.trim() || `strict codesign verification failed for ${target}`);
  expect(!output.includes("code has no resources but signature indicates they must be present"), "bundle resource seal is invalid");
}

function machOBinaries(bundlePath) {
  const files = command("find", [join(bundlePath, "Contents"), "-type", "f", "-perm", "-111", "-print"]).trim().split("\n").filter(Boolean);
  return files.filter((file) => command("file", ["-b", file]).includes("Mach-O"));
}

function verifyPlist(plist) {
  expect(plist.CFBundleIdentifier === "ai.desktoplab.desktop", "CFBundleIdentifier must be ai.desktoplab.desktop");
  expect(plist.LSApplicationCategoryType === "public.app-category.developer-tools", "bundle category must be DeveloperTool");
  expect(plist.LSMinimumSystemVersion === "13.0", "minimum macOS version must be explicit");
  expect((plist.CFBundleURLTypes ?? []).some((entry) => (entry.CFBundleURLSchemes ?? []).includes("desktoplab")), "desktoplab:// URL scheme must be registered");
  expect((plist.CFBundleDocumentTypes ?? []).some((entry) => (entry.LSItemContentTypes ?? []).includes("public.folder")), "public.folder association must be registered");
  for (const key of ["NSDesktopFolderUsageDescription", "NSDocumentsFolderUsageDescription", "NSDownloadsFolderUsageDescription"]) {
    expect(typeof plist[key] === "string" && plist[key].length > 20, `${key} must be present and meaningful`);
  }
  for (const key of ["NSMicrophoneUsageDescription", "NSCameraUsageDescription", "NSSpeechRecognitionUsageDescription", "NSBluetoothAlwaysUsageDescription", "NSBluetoothPeripheralUsageDescription"]) {
    expect(!(key in plist), `${key} is forbidden without an implemented feature`);
  }
}

function command(executable, values) {
  return execFileSync(executable, values, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}

function expect(condition, message) {
  if (!condition) throw new Error(message);
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] === "--app") parsed.app = values[++index];
    else if (values[index] === "--dmg") parsed.dmg = values[++index];
    else if (values[index] === "--mode") parsed.mode = values[++index];
    else if (values[index] === "--architecture") parsed.architecture = values[++index];
  }
  if (parsed.mode && !["dev", "release", "notarized"].includes(parsed.mode)) throw new Error(`unsupported verification mode: ${parsed.mode}`);
  return parsed;
}
