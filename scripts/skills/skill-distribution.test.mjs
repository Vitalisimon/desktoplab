import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import { join } from "node:path";
import test from "node:test";

import { syncDistribution, validateDistribution } from "./skill-distribution-core.mjs";

test("valid skills sync idempotently and repo instructions remain authoritative", () => {
  const fixture = createFixture();
  const first = syncDistribution(fixture.manifest, fixture.path);
  const second = syncDistribution(fixture.manifest, fixture.path);
  assert.equal(first.status, "pass");
  assert.equal(first.changed, true);
  assert.equal(second.changed, false);
  assert.deepEqual(second.precedence, ["repo", "user", "global"]);
  assert.match(readFileSync(join(fixture.root, "codex", "skills", "review", "SKILL.md"), "utf8"), /Review repository changes/);
});

test("sync prunes only stale entries managed by the same manifest", () => {
  const fixture = createFixture();
  const stale = join(fixture.root, "codex", "skills", "stale");
  const unmanaged = join(fixture.root, "codex", "skills", "user-owned");
  mkdirSync(stale, { recursive: true });
  mkdirSync(unmanaged, { recursive: true });
  writeFileSync(join(stale, ".desktoplab-managed.json"), JSON.stringify({ manifestId: fixture.manifest.id }));
  writeFileSync(join(unmanaged, "SKILL.md"), "unmanaged");
  const report = syncDistribution(fixture.manifest, fixture.path);
  assert.ok(report.operations.includes("pruned:codex:stale"));
  assert.equal(readFileSync(join(unmanaged, "SKILL.md"), "utf8"), "unmanaged");
});

test("broken metadata paths collisions and unsupported discovery fail before sync", () => {
  const fixture = createFixture();
  fixture.manifest.clients[0].discoveryRule = "plugins/{name}.md";
  fixture.manifest.skills[0].scripts = ["scripts/missing.sh"];
  fixture.manifest.skills.push({ ...fixture.manifest.skills[0], owner: "user" });
  const report = validateDistribution(fixture.manifest, fixture.path);
  assert.equal(report.status, "blocked");
  assert.ok(report.failures.includes("unsupported_discovery_rule:codex"));
  assert.ok(report.failures.includes("broken_skill_script:review:scripts/missing.sh"));
  assert.ok(report.failures.includes("skill_collision:review:repo,user"));
});

test("broken local links inside skill instructions are reported automatically", () => {
  const fixture = createFixture();
  writeFileSync(join(fixture.root, "canonical", "review", "SKILL.md"), "---\nname: review\ndescription: Review repository changes with durable evidence.\n---\n\n[Missing guide](references/missing.md)\n");
  const report = validateDistribution(fixture.manifest, fixture.path);
  assert.ok(report.failures.includes("broken_skill_path:review:references/missing.md"));
});

test("unmanaged destinations are never overwritten", () => {
  const fixture = createFixture();
  const destination = join(fixture.root, "codex", "skills", "review");
  mkdirSync(destination, { recursive: true });
  writeFileSync(join(destination, "SKILL.md"), "user owned");
  assert.throws(() => syncDistribution(fixture.manifest, fixture.path), /unmanaged_destination_collision/);
});

function createFixture() {
  const root = mkdtempSync(join(os.tmpdir(), "desktoplab-skills-"));
  mkdirSync(join(root, "canonical", "review", "scripts"), { recursive: true });
  writeFileSync(join(root, "AGENTS.md"), "# Repo instructions\n");
  writeFileSync(join(root, "canonical", "review", "SKILL.md"), "---\nname: review\ndescription: Review repository changes with evidence and explicit findings.\n---\n\n# Review repository changes\n");
  writeFileSync(join(root, "canonical", "review", "scripts", "check.sh"), "exit 0\n");
  const manifest = {
    kind: "desktoplab.skill-distribution",
    schemaVersion: 1,
    id: "fixture.skills",
    clients: [{ id: "codex", root: "codex", discoveryRule: "skills/{name}/SKILL.md" }],
    instructions: [{ id: "repo", owner: "repo", path: "AGENTS.md", authoritative: true }],
    skills: [{ name: "review", owner: "repo", source: "canonical/review", clients: ["codex"], scripts: ["scripts/check.sh"], referencedPaths: [] }],
  };
  const path = join(root, "distribution.json");
  writeFileSync(path, JSON.stringify(manifest));
  return { root, path, manifest };
}
