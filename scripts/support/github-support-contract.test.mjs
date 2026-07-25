import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "yaml";

const read = (path) => readFileSync(path, "utf8");
const parseForm = (path) => parse(read(path));

test("GitHub issue intake routes public and confidential reports correctly", () => {
  const config = parseForm(".github/ISSUE_TEMPLATE/config.yml");
  assert.equal(config.blank_issues_enabled, false);
  assert.equal(config.contact_links.length, 3);

  const urls = config.contact_links.map((link) => link.url);
  assert.ok(urls.some((url) => url.endsWith("/discussions/categories/q-a")));
  assert.ok(urls.some((url) => url.endsWith("/security/advisories/new")));
  assert.equal(existsSync(".github/ISSUE_TEMPLATE/security.md"), false);
});

test("bug form collects reproducible evidence behind an explicit privacy check", () => {
  const form = parseForm(".github/ISSUE_TEMPLATE/bug_report.yml");
  assert.deepEqual(form.labels, ["bug", "triage"]);

  const fields = new Map(form.body.filter((item) => item.id).map((item) => [item.id, item]));
  for (const id of ["version", "package", "operating-system", "operating-system-version", "architecture", "impact", "problem", "reproduction", "expected"]) {
    assert.equal(fields.get(id)?.validations?.required, true, `${id} must be required`);
  }
  assert.ok(!fields.get("package").attributes.options.includes("Windows installer"));

  const privacyOptions = fields.get("privacy")?.attributes?.options ?? [];
  assert.equal(privacyOptions.length, 2);
  assert.ok(privacyOptions.every((option) => option.required));
  assert.match(form.body[0].attributes.value, /Private Vulnerability Reporting/);
});

test("installation form collects exact public package evidence without trust bypasses", () => {
  const form = parseForm(".github/ISSUE_TEMPLATE/installation_problem.yml");
  assert.deepEqual(form.labels, ["bug", "triage", "area: installation"]);

  const fields = new Map(form.body.filter((item) => item.id).map((item) => [item.id, item]));
  for (const id of ["version", "package", "operating-system", "hardware", "stage", "problem", "reproduction", "expected"]) {
    assert.equal(fields.get(id)?.validations?.required, true, `${id} must be required`);
  }
  assert.deepEqual(fields.get("package").attributes.options, [
    "macOS Apple Silicon DMG",
    "Linux x64 AppImage",
    "Linux x64 deb",
    "Linux x64 rpm",
  ]);
  const checks = fields.get("checks").attributes.options;
  assert.equal(checks.length, 4);
  assert.ok(checks.every((option) => option.required));
  assert.match(form.body[0].attributes.value, /Never bypass Gatekeeper/);
});

test("feature form starts from a user problem and prevents private-data intake", () => {
  const form = parseForm(".github/ISSUE_TEMPLATE/feature_request.yml");
  const fields = new Map(form.body.filter((item) => item.id).map((item) => [item.id, item]));
  assert.equal(fields.get("problem")?.validations?.required, true);
  assert.equal(fields.get("outcome")?.validations?.required, true);
  assert.equal(fields.get("area")?.validations?.required, true);
  assert.ok(fields.get("scope")?.attributes?.options.every((option) => option.required));
});

test("public support policy documents every intake channel and audit boundary", () => {
  const root = read("SUPPORT.md");
  const publicGuide = read("docs-public/support.md");
  for (const text of [root, publicGuide]) {
    assert.match(text, /GitHub Discussions/);
    assert.match(text, /Bug/);
    assert.match(text, /Feature/);
    assert.match(text, /Private Vulnerability Reporting/);
    assert.match(text, /diagnostic/i);
  }
  assert.match(publicGuide, /ignored `dist\/support-audit\/`/);
  assert.match(publicGuide, /must never be committed/);
  assert.match(read("docs-public/triage.md"), /release blocker/);

  const publicGate = read("docs-public/public-export-gate.md");
  assert.match(publicGate, /Private Vulnerability Reporting is enabled/);
  assert.match(publicGate, /support:audit:github -- --require issues,discussions,advisories,pvr/);
});

test("support intake files stay reviewable", () => {
  for (const [path, limit] of [
    [".github/ISSUE_TEMPLATE/bug_report.yml", 120],
    [".github/ISSUE_TEMPLATE/feature_request.yml", 90],
    [".github/ISSUE_TEMPLATE/installation_problem.yml", 110],
    ["scripts/support/github-support-contract.test.mjs", 150],
  ]) {
    const logicalLines = read(path).split("\n").filter((line) => line.trim()).length;
    assert.ok(logicalLines <= limit, `${path} has ${logicalLines} logical lines, limit ${limit}`);
  }
});
