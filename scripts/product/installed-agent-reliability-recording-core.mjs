import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { buildRunDescriptors } from "./agent-reliability-campaign-core.mjs";
import { installedAgentFixture, installedAgentPrompts } from "./installed-agent-recording-core.mjs";
import { prepareProfileFiles, profileForSeed, reliabilityProfilesBySeed } from "./agent-reliability-profiles.mjs";

export { deriveReliabilityConfiguration, localModelProvenance } from "./agent-reliability-configuration.mjs";

export const reliabilityCases = Object.freeze(Object.keys(installedAgentPrompts)), reliabilitySeeds = Object.freeze([7, 19, 43, 71, 97]);

export function reliabilityDriverPlan() {
  return reliabilityCases.flatMap((caseId) => reliabilitySeeds.map((seed) => ({
    caseId,
    seed,
    profileId: profileForSeed(seed),
    repetition: 1,
    prompt: installedAgentPrompts[caseId],
    approvalMayBeRequired: ["create", "patch", "test_repair"].includes(caseId),
  })));
}

export function createReliabilityManifest({ candidateId, appHash, configuration }) {
  return {
    kind: "desktoplab.agent-reliability-manifest",
    schemaVersion: 1,
    campaignId: "installed-macos-agent-reliability-v3",
    candidateId,
    appHash,
    cases: reliabilityCases,
    seeds: reliabilitySeeds,
    profilesBySeed: reliabilityProfilesBySeed,
    repetitions: 1,
    timeoutMs: 12 * 60_000,
    minimumPassRate: 0.9,
    configuration,
  };
}

export function reliabilityDescriptors(manifest) {
  return buildRunDescriptors(manifest);
}

export function prepareReliabilityWorkspace(path, caseId, profileId = "medium") {
  const target = resolve(path);
  mkdirSync(target, { recursive: true });
  writeFileSync(join(target, "calculator.js"), installedAgentFixture.initialImplementationContent);
  writeFileSync(join(target, "calculator.test.js"), installedAgentFixture.protectedContent);
  writeFileSync(join(target, "package.json"), installedAgentFixture.packageContent);
  writeFileSync(join(target, "release-note.md"), installedAgentFixture.initialPatchedContent);
  prepareProfileFiles(target, profileId);
  git(target, "init", "-b", "main");
  git(target, "add", "--all");
  git(target, "-c", "user.name=DesktopLab", "-c", "user.email=fixture@desktoplab.local", "commit", "-m", "reliability fixture");
  if (caseId === "diff") writeFileSync(join(target, installedAgentFixture.patchedPath), installedAgentFixture.patchedContent);
  return target;
}

export function snapshotSqlite(source, destination) {
  const origin = resolve(source);
  const target = resolve(destination);
  if (relative(origin, target) === "") throw new Error("state snapshot destination must differ from source");
  mkdirSync(resolve(target, ".."), { recursive: true });
  const escaped = target.replaceAll("'", "''");
  const database = new DatabaseSync(origin, { readOnly: true });
  try {
    database.exec(`vacuum into '${escaped}'`);
  } finally { database.close(); }
  return target;
}

export function snapshotReliabilityState(source, destination) {
  const target = snapshotSqlite(source, destination);
  const database = new DatabaseSync(target);
  try {
    const ftsSchema = database.prepare("select sql from sqlite_schema where type = 'table' and name = 'support_records_fts'").get()?.sql ?? null;
    const virtualFts = /^CREATE VIRTUAL TABLE\s+support_records_fts\s+USING\s+fts5\s*\(/i.test(ftsSchema ?? "");
    if (ftsSchema && !virtualFts && /^CREATE VIRTUAL TABLE/i.test(ftsSchema)) throw new Error("unsupported support search virtual table");
    const statements = [
      "begin immediate;",
      "delete from event_log;",
      "delete from productization_state where kind not in ('backend_readiness','setup_state','setup_pipeline','runtime_inventory','model_inventory');",
      "delete from settings where key not in ('approval.default_mode','routing.selected_route_id');",
      "delete from support_records;",
      "delete from support_sync_state;",
      "delete from support_tombstones;",
    ];
    if (!virtualFts && ftsSchema) statements.push("delete from support_records_fts;");
    database.exec(statements.join(" "));
    if (virtualFts) removeRegenerableFtsSchema(database);
    database.exec("commit; vacuum;");
  } catch (error) {
    try { database.exec("pragma writable_schema = off; rollback;"); } catch {}
    throw error;
  } finally { database.close(); }
  return target;
}

function removeRegenerableFtsSchema(database) {
  const version = database.prepare("pragma schema_version").get().schema_version;
  database.prepare("delete from schema_migrations where version = ?").run(4);
  database.exec("pragma writable_schema = on;");
  const removed = database.prepare("delete from sqlite_schema where name = ? or name glob ?").run("support_records_fts", "support_records_fts_*").changes;
  database.exec("pragma writable_schema = off;");
  if (removed < 2) throw new Error("support search index schema was not fully removed");
  database.exec(`pragma schema_version = ${version + 1};`);
}

function git(cwd, ...args) { execFileSync("git", args, { cwd, stdio: "ignore" }); }
function digest(value) { return `sha256:${createHash("sha256").update(value).digest("hex")}`; }
