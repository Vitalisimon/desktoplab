import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

export function sha256File(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

export function hashArtifact(artifactPath) {
  const stat = fs.lstatSync(artifactPath);
  if (!stat.isDirectory()) return { sha256: sha256File(artifactPath), sizeBytes: stat.size };
  const hash = crypto.createHash("sha256");
  let sizeBytes = 0;
  for (const entry of walkBundle(artifactPath)) {
    const relative = path.relative(artifactPath, entry).split(path.sep).join("/");
    const item = fs.lstatSync(entry);
    if (item.isSymbolicLink()) {
      hash.update(`link\0${relative}\0${fs.readlinkSync(entry)}\0`);
    } else {
      const bytes = fs.readFileSync(entry);
      sizeBytes += bytes.length;
      hash.update(`file\0${relative}\0${item.mode & 0o777}\0${bytes.length}\0`);
      hash.update(bytes);
      hash.update("\0");
    }
  }
  return { sha256: hash.digest("hex"), sizeBytes };
}

export function writeArtifactEvidence({ root, evidenceDir, artifactPaths, build, signatureStateFor = () => "unverified_dev" }) {
  const entries = artifactPaths.sort().map((artifactPath) => {
    const absolutePath = path.resolve(root, artifactPath);
    const digest = hashArtifact(absolutePath);
    return {
      relativePath: path.relative(root, absolutePath),
      fileName: path.basename(absolutePath),
      kind: absolutePath.endsWith(".app") ? "app_bundle" : "distribution_file",
      target: targetFor(absolutePath, build.architecture),
      channel: build.channel,
      version: build.version,
      ...digest,
      signatureState: signatureStateFor(absolutePath),
    };
  });
  if (!entries.some((entry) => entry.kind === "app_bundle") && process.platform === "darwin") {
    throw new Error("macOS artifact evidence requires the exact .app bundle");
  }
  const manifest = { kind: "desktoplab.artifact-provenance", schemaVersion: 2, build, entries };
  fs.mkdirSync(evidenceDir, { recursive: true });
  fs.writeFileSync(path.join(evidenceDir, "artifact-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  fs.writeFileSync(path.join(evidenceDir, "dev-artifacts.txt"), `${entries.map((entry) => entry.relativePath).join("\n")}\n`);
  fs.writeFileSync(path.join(evidenceDir, "SHA256SUMS.txt"), `${entries.map((entry) => `${entry.sha256}  ${entry.fileName}`).join("\n")}\n`);
  return manifest;
}

export function verifyArtifactEvidence({ root, evidenceDir, currentHead, currentTreeState, installedAppPath = null }) {
  const manifest = JSON.parse(fs.readFileSync(path.join(evidenceDir, "artifact-manifest.json"), "utf8"));
  if (manifest.kind !== "desktoplab.artifact-provenance" || manifest.schemaVersion !== 2) throw new Error("unexpected artifact manifest contract");
  if (manifest.build.commitSha !== currentHead) throw new Error(`artifact commit ${manifest.build.commitSha} differs from HEAD ${currentHead}`);
  if (manifest.build.treeState !== "clean" || currentTreeState !== "clean") throw new Error("current-head artifact verification requires a clean source tree");
  verifyLockfiles(root, manifest.build.lockfiles);
  const listed = fs.readFileSync(path.join(evidenceDir, "dev-artifacts.txt"), "utf8").trim().split(/\r?\n/).filter(Boolean);
  const sums = checksumMap(path.join(evidenceDir, "SHA256SUMS.txt"));
  if (listed.length !== manifest.entries.length || sums.size !== manifest.entries.length) throw new Error("artifact list, manifest and checksum counts differ");
  for (const entry of manifest.entries) {
    if (!listed.includes(entry.relativePath)) throw new Error(`artifact list is missing ${entry.relativePath}`);
    const actual = hashArtifact(path.resolve(root, entry.relativePath));
    if (actual.sha256 !== entry.sha256 || actual.sizeBytes !== entry.sizeBytes) throw new Error(`artifact mutated after manifest creation: ${entry.fileName}`);
    if (sums.get(entry.fileName) !== entry.sha256) throw new Error(`SHA256SUMS mismatch for ${entry.fileName}`);
    if (entry.kind === "app_bundle") verifyEmbeddedBuild(path.resolve(root, entry.relativePath), manifest.build);
  }
  if (installedAppPath) verifyInstalledApp(installedAppPath, manifest, currentHead);
  return manifest;
}

export function readEmbeddedBuild(appPath) {
  const metadataPath = path.join(appPath, "Contents", "Resources", "DesktopLabBuild.json");
  if (!fs.existsSync(metadataPath)) throw new Error(`app build metadata is missing: ${metadataPath}`);
  return JSON.parse(fs.readFileSync(metadataPath, "utf8"));
}

function verifyInstalledApp(installedAppPath, manifest, currentHead) {
  if (!fs.existsSync(installedAppPath)) throw new Error(`installed app is missing: ${installedAppPath}`);
  const metadata = readEmbeddedBuild(installedAppPath);
  if (metadata.commitSha !== currentHead) throw new Error(`installed app commit ${metadata.commitSha} differs from HEAD ${currentHead}`);
  const candidate = manifest.entries.find((entry) => entry.kind === "app_bundle");
  if (!candidate) throw new Error("manifest has no app bundle to compare with installed app");
  if (hashArtifact(installedAppPath).sha256 !== candidate.sha256) throw new Error("installed app hash differs from the current-head candidate");
}

function verifyEmbeddedBuild(appPath, build) {
  const metadata = readEmbeddedBuild(appPath);
  for (const key of ["version", "commitSha", "channel", "treeState", "architecture", "runner"]) {
    if (metadata[key] !== build[key]) throw new Error(`embedded app ${key} differs from artifact manifest`);
  }
  if (JSON.stringify(metadata.lockfiles) !== JSON.stringify(build.lockfiles)) throw new Error("embedded app lock hashes differ from artifact manifest");
}

function verifyLockfiles(root, lockfiles) {
  if (!Array.isArray(lockfiles) || lockfiles.length === 0) throw new Error("artifact manifest has no dependency lock hashes");
  for (const lock of lockfiles) {
    if (sha256File(path.join(root, lock.path)) !== lock.sha256) throw new Error(`dependency lock changed after build: ${lock.path}`);
  }
}

function checksumMap(filePath) {
  return new Map(fs.readFileSync(filePath, "utf8").trim().split(/\r?\n/).filter(Boolean).map((line) => {
    const match = line.match(/^([a-f0-9]{64})  (.+)$/);
    if (!match) throw new Error(`invalid checksum line: ${line}`);
    return [match[2], match[1]];
  }));
}

function walkBundle(root) {
  return fs.readdirSync(root, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile() || entry.isSymbolicLink())
    .map((entry) => path.join(entry.parentPath, entry.name))
    .sort();
}

function targetFor(artifactPath, architecture) {
  if (artifactPath.endsWith(".app") || artifactPath.endsWith(".dmg")) return `macos-${architecture === "arm64" ? "aarch64" : architecture}`;
  if (/\.(AppImage|deb|rpm)$/.test(artifactPath)) return `linux-${architecture}`;
  if (/\.(exe|msi)$/.test(artifactPath)) return `windows-${architecture}`;
  throw new Error(`unsupported artifact type: ${artifactPath}`);
}
