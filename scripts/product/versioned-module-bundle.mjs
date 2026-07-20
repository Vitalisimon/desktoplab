import { createHash } from "node:crypto";
import { readFileSync, realpathSync, statSync } from "node:fs";
import { basename, dirname, relative, resolve, sep } from "node:path";

import { init, parse } from "es-module-lexer";

export async function versionedModuleBundle(entryPath, boundaryPath) {
  await init;
  const boundary = canonicalDirectory(boundaryPath);
  const entry = canonicalFile(entryPath);
  assertWithinBoundary(entry, boundary);
  const visited = new Map();
  await visit(entry, boundary, visited);
  const sources = [...visited.values()].sort((left, right) => (
    left.path === relativePath(boundary, entry) ? -1
      : right.path === relativePath(boundary, entry) ? 1
        : left.path.localeCompare(right.path)
  ));
  return {
    schemaVersion: 1,
    id: basename(entry),
    entrySha256: sources[0].sha256,
    bundleSha256: moduleSourceBundleDigest(sources),
    sourceCount: sources.length,
    sources,
  };
}

export function moduleSourceBundleDigest(sources) {
  return digest([...sources]
    .sort((left, right) => left.path.localeCompare(right.path))
    .map((source) => `${source.path}\0${source.sha256}`)
    .join("\0"));
}

async function visit(path, boundary, visited) {
  if (visited.has(path)) return;
  const content = readFileSync(path, "utf8");
  visited.set(path, { path: relativePath(boundary, path), sha256: digest(content) });
  const [imports] = parse(content);
  for (const item of imports) {
    if (item.d >= 0 && item.n == null) throw new Error(`computed dynamic import is not attestable: ${relativePath(boundary, path)}`);
    const specifier = item.n;
    if (!specifier || (!specifier.startsWith(".") && !specifier.startsWith("/") && !specifier.startsWith("file:"))) continue;
    if (!specifier.startsWith(".")) throw new Error(`absolute local import is not attestable: ${specifier}`);
    const dependency = canonicalFile(resolve(dirname(path), specifier));
    assertWithinBoundary(dependency, boundary);
    await visit(dependency, boundary, visited);
  }
}

function assertWithinBoundary(path, boundary) {
  const local = relative(boundary, path);
  if (!local || local.startsWith(`..${sep}`) || local === ".." || resolve(boundary, local) !== path) {
    throw new Error(`${path} is outside the attested module boundary`);
  }
}

function canonicalFile(path) {
  const target = realpathSync(resolve(path));
  if (!statSync(target).isFile()) throw new Error(`${path} is not a file`);
  return target;
}

function canonicalDirectory(path) {
  const target = realpathSync(resolve(path));
  if (!statSync(target).isDirectory()) throw new Error(`${path} is not a directory`);
  return target;
}

function relativePath(boundary, path) { return relative(boundary, path).split(sep).join("/"); }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
