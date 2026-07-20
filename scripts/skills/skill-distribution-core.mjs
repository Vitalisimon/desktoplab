import { createHash } from "node:crypto";
import {
  cpSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { parse as parseYaml } from "yaml";

const PRECEDENCE = { global: 100, user: 200, repo: 300 };
const CLIENT_RULES = new Map([
  ["desktoplab", "skills/{name}/SKILL.md"],
  ["codex", "skills/{name}/SKILL.md"],
  ["claude", "skills/{name}/SKILL.md"],
]);

export function validateDistribution(manifest, manifestPath) {
  const failures = [];
  const root = dirname(resolve(manifestPath));
  if (manifest?.kind !== "desktoplab.skill-distribution" || manifest?.schemaVersion !== 1) failures.push("invalid_manifest_contract");
  if (!/^[a-z0-9][a-z0-9._-]+$/.test(manifest?.id ?? "")) failures.push("invalid_manifest_id");
  const clients = new Map();
  for (const client of manifest?.clients ?? []) {
    const expected = CLIENT_RULES.get(client.id);
    if (!expected) failures.push(`unsupported_client:${client.id ?? "missing"}`);
    else if (client.discoveryRule !== expected) failures.push(`unsupported_discovery_rule:${client.id}`);
    if (!safeRelative(client.root)) failures.push(`unsafe_client_root:${client.id ?? "missing"}`);
    if (clients.has(client.id)) failures.push(`duplicate_client:${client.id}`);
    clients.set(client.id, client);
  }
  const names = new Map();
  for (const skill of manifest?.skills ?? []) {
    validateSkill(skill, root, clients, failures);
    const owners = names.get(skill.name) ?? [];
    owners.push(skill.owner);
    names.set(skill.name, owners);
  }
  for (const [name, owners] of names) if (owners.length > 1) failures.push(`skill_collision:${name}:${owners.join(",")}`);
  validateInstructions(manifest?.instructions ?? [], root, failures);
  return {
    kind: "desktoplab.skill-distribution-validation",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "blocked",
    failures: [...new Set(failures)].sort(),
    precedence: Object.entries(PRECEDENCE).sort((left, right) => right[1] - left[1]).map(([owner]) => owner),
  };
}

export function syncDistribution(manifest, manifestPath) {
  const validation = validateDistribution(manifest, manifestPath);
  if (validation.status !== "pass") return { ...validation, changed: false, operations: [] };
  const root = dirname(resolve(manifestPath));
  const expectedByClient = new Map();
  const operations = [];
  for (const skill of manifest.skills ?? []) for (const clientId of skill.clients) {
    const client = manifest.clients.find((candidate) => candidate.id === clientId);
    const clientRoot = resolveInside(root, client.root);
    const destination = join(clientRoot, "skills", skill.name);
    const source = resolveInside(root, skill.source);
    const sourceHash = treeHash(source);
    const expected = expectedByClient.get(clientId) ?? new Set();
    expected.add(skill.name);
    expectedByClient.set(clientId, expected);
    const markerPath = join(destination, ".desktoplab-managed.json");
    const marker = readJson(markerPath);
    if (existsSync(destination) && marker?.manifestId !== manifest.id) throw new Error(`unmanaged_destination_collision:${clientId}:${skill.name}`);
    if (marker?.sourceHash === sourceHash && existsSync(join(destination, "SKILL.md"))) continue;
    if (existsSync(destination)) rmSync(destination, { recursive: true, force: true });
    mkdirSync(destination, { recursive: true, mode: 0o700 });
    cpSync(source, destination, { recursive: true, dereference: false });
    writeFileSync(markerPath, `${JSON.stringify({ manifestId: manifest.id, skill: skill.name, owner: skill.owner, sourceHash }, null, 2)}\n`, { mode: 0o600 });
    operations.push(`updated:${clientId}:${skill.name}`);
  }
  for (const client of manifest.clients ?? []) pruneManaged(root, client, manifest.id, expectedByClient.get(client.id) ?? new Set(), operations);
  return { ...validation, changed: operations.length > 0, operations };
}

function validateSkill(skill, root, clients, failures) {
  if (!/^[a-z0-9][a-z0-9-]{1,63}$/.test(skill?.name ?? "")) failures.push(`invalid_skill_name:${skill?.name ?? "missing"}`);
  if (!Object.hasOwn(PRECEDENCE, skill?.owner)) failures.push(`invalid_skill_owner:${skill?.name ?? "missing"}`);
  if (!safeRelative(skill?.source)) return failures.push(`unsafe_skill_source:${skill?.name ?? "missing"}`);
  const source = resolveInside(root, skill.source);
  const skillFile = join(source, "SKILL.md");
  if (!existsSync(skillFile)) return failures.push(`missing_skill_file:${skill.name}`);
  rejectSymlinks(source, failures, skill.name);
  const sourceText = readFileSync(skillFile, "utf8");
  const metadata = skillMetadata(sourceText);
  if (metadata.name !== skill.name) failures.push(`skill_name_mismatch:${skill.name}`);
  if (typeof metadata.description !== "string" || metadata.description.trim().length < 20) failures.push(`trigger_description_missing:${skill.name}`);
  for (const path of [...(skill.referencedPaths ?? []), ...localMarkdownLinks(sourceText)]) {
    if (!safeRelative(path) || !existsSync(resolveInside(source, path))) failures.push(`broken_skill_path:${skill.name}:${path}`);
  }
  for (const path of skill.scripts ?? []) {
    const script = safeRelative(path) ? resolveInside(source, path) : null;
    if (!script || !existsSync(script) || !lstatSync(script).isFile()) failures.push(`broken_skill_script:${skill.name}:${path}`);
  }
  for (const client of skill.clients ?? []) if (!clients.has(client)) failures.push(`unknown_skill_client:${skill.name}:${client}`);
}

function validateInstructions(instructions, root, failures) {
  let highest = -1;
  let authoritativeOwner = null;
  const ids = new Set();
  for (const instruction of instructions) {
    if (ids.has(instruction.id)) failures.push(`duplicate_instruction:${instruction.id}`);
    ids.add(instruction.id);
    if (!Object.hasOwn(PRECEDENCE, instruction.owner)) failures.push(`invalid_instruction_owner:${instruction.id}`);
    if (!safeRelative(instruction.path) || !existsSync(resolveInside(root, instruction.path))) failures.push(`broken_instruction_path:${instruction.id}`);
    if (instruction.authoritative && PRECEDENCE[instruction.owner] > highest) {
      highest = PRECEDENCE[instruction.owner];
      authoritativeOwner = instruction.owner;
    }
  }
  if (instructions.length > 0 && authoritativeOwner !== "repo") failures.push("repo_instructions_not_authoritative");
}

function pruneManaged(root, client, manifestId, expected, operations) {
  const skillsRoot = join(resolveInside(root, client.root), "skills");
  if (!existsSync(skillsRoot)) return;
  for (const entry of readdirSync(skillsRoot, { withFileTypes: true })) {
    if (!entry.isDirectory() || expected.has(entry.name)) continue;
    const destination = join(skillsRoot, entry.name);
    if (readJson(join(destination, ".desktoplab-managed.json"))?.manifestId !== manifestId) continue;
    rmSync(destination, { recursive: true, force: true });
    operations.push(`pruned:${client.id}:${entry.name}`);
  }
}

function treeHash(root) {
  const hash = createHash("sha256");
  for (const path of walk(root)) {
    hash.update(relative(root, path));
    hash.update(readFileSync(path));
  }
  return `sha256:${hash.digest("hex")}`;
}

function walk(root) {
  const files = [];
  for (const entry of readdirSync(root, { withFileTypes: true }).sort((left, right) => left.name.localeCompare(right.name))) {
    const path = join(root, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`skill_symlink_not_allowed:${path}`);
    if (entry.isDirectory()) files.push(...walk(path));
    else if (entry.isFile()) files.push(path);
  }
  return files;
}

function skillMetadata(source) {
  const match = source.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  if (!match) return {};
  try { return parseYaml(match[1]) ?? {}; } catch { return {}; }
}

function localMarkdownLinks(source) {
  const links = [];
  for (const match of source.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
    const path = match[1].trim().split(/\s+/)[0];
    if (!path.startsWith("#") && !/^[a-z][a-z0-9+.-]*:/i.test(path)) links.push(decodeURIComponent(path));
  }
  return links;
}

function rejectSymlinks(root, failures, name) {
  try { walk(root); } catch { failures.push(`skill_symlink_not_allowed:${name}`); }
}

function safeRelative(value) {
  return typeof value === "string" && value.length > 0 && !value.startsWith("/") && !value.split(/[\\/]/).includes("..");
}

function resolveInside(root, value) {
  const path = resolve(root, value);
  if (path !== root && !path.startsWith(`${root}${sep}`)) throw new Error("path_outside_distribution_root");
  return path;
}

function readJson(path) {
  try { return JSON.parse(readFileSync(path, "utf8")); } catch { return null; }
}
