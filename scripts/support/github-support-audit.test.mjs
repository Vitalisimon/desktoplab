import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  buildSnapshot,
  collectConnection,
  evaluateRequirements,
  flattenRestPages,
  publicSummary,
} from "./github-support-audit-core.mjs";
import { collectDiscussions, collectIssues } from "./github-support-audit.mjs";

test("REST page collection preserves every record", () => {
  assert.deepEqual(flattenRestPages([[{ id: 1 }], [{ id: 2 }], []]), [{ id: 1 }, { id: 2 }]);
  assert.deepEqual(flattenRestPages([{ id: 1 }]), [{ id: 1 }]);
});

test("connection collection follows every cursor and rejects cursor loops", async () => {
  const pages = new Map([
    [null, { nodes: [{ id: 1 }], pageInfo: { hasNextPage: true, endCursor: "one" } }],
    ["one", { nodes: [{ id: 2 }], pageInfo: { hasNextPage: false, endCursor: "two" } }],
  ]);
  const result = await collectConnection((cursor) => pages.get(cursor));
  assert.deepEqual(result, { nodes: [{ id: 1 }, { id: 2 }], pages: 2, complete: true });

  await assert.rejects(
    collectConnection(() => ({ nodes: [], pageInfo: { hasNextPage: true, endCursor: "same" } })),
    /INVALID_PAGINATION_CURSOR/,
  );
});

test("issue collector excludes pull requests and captures comments plus timeline", async () => {
  const client = {
    restAll(path) {
      if (path.includes("state=all")) return [{ number: 1, body: "issue" }, { number: 2, pull_request: {} }];
      if (path.includes("comments")) return [{ id: 10, body: "comment" }];
      if (path.includes("timeline")) return [{ id: 20, event: "closed" }];
      throw new Error(path);
    },
  };
  const issues = await collectIssues(client, "owner/repo");
  assert.equal(issues.length, 1);
  assert.equal(issues[0].comments[0].body, "comment");
  assert.equal(issues[0].timeline[0].event, "closed");
});

test("discussion collector captures all comments and nested replies", async () => {
  const client = {
    graphql(query, variables) {
      if (query.includes("discussions(first")) {
        return { repository: { discussions: { nodes: [{ id: "D1", body: "topic" }], pageInfo: { hasNextPage: false } } } };
      }
      if (query.includes("... on Discussion {")) {
        return { node: { comments: { nodes: [{ id: "C1", body: "answer" }], pageInfo: { hasNextPage: false } } } };
      }
      assert.equal(variables.id, "C1");
      return { node: { replies: { nodes: [{ id: "R1", body: "reply" }], pageInfo: { hasNextPage: false } } } };
    },
  };
  const discussions = await collectDiscussions(client, "owner", "repo");
  assert.equal(discussions[0].comments[0].replies[0].body, "reply");
  assert.equal(discussions[0].auditCoverage.comments, "complete");
});

test("snapshot exposes only counts and coverage in its public summary", () => {
  const snapshot = buildSnapshot({
    repository: { full_name: "owner/repo", html_url: "https://example.test", visibility: "private", default_branch: "main", has_issues: true, has_discussions: true },
    sourceCommit: "abc",
    access: {
      issues: { state: "complete" },
      discussions: { state: "complete" },
      advisories: { state: "complete" },
      privateVulnerabilityReporting: "requires-public-repository",
    },
    issues: [{ comments: [{ body: "PRIVATE_ISSUE_BODY" }], timeline: [] }],
    discussions: [{ comments: [{ replies: [{ body: "CONFIDENTIAL_REPLY_BODY" }] }] }],
    advisories: [{ description: "SECRET_ADVISORY_BODY" }],
  });
  const findings = evaluateRequirements(snapshot, ["issues", "discussions", "pvr"]);
  assert.deepEqual(findings, ["pvr:requires-public-repository"]);
  const summary = JSON.stringify(publicSummary(snapshot, "/ignored/file.json", findings));
  assert.doesNotMatch(summary, /PRIVATE_ISSUE_BODY|CONFIDENTIAL_REPLY_BODY|SECRET_ADVISORY_BODY/);
  assert.equal(snapshot.counts.discussionReplies, 1);
});

test("requirements distinguish disabled features from collection failures", () => {
  const snapshot = buildSnapshot({
    repository: { full_name: "owner/repo", html_url: "https://example.test", visibility: "private", default_branch: "main", has_issues: true, has_discussions: false },
    sourceCommit: "abc",
    access: {
      issues: { state: "complete" },
      discussions: { state: "complete" },
      advisories: { state: "requires-public-repository" },
      privateVulnerabilityReporting: "requires-public-repository",
    },
    issues: [],
    discussions: [],
    advisories: [],
  });
  assert.deepEqual(
    evaluateRequirements(snapshot, ["issues", "discussions", "advisories"]),
    ["discussions:disabled", "advisories:requires-public-repository"],
  );
});

test("support audit implementation stays reviewable", () => {
  for (const [path, limit] of [
    ["scripts/support/github-support-audit-core.mjs", 180],
    ["scripts/support/github-support-audit.mjs", 320],
    ["scripts/support/github-support-audit.test.mjs", 180],
  ]) {
    const logicalLines = readFileSync(path, "utf8").split("\n").filter((line) => line.trim()).length;
    assert.ok(logicalLines <= limit, `${path} has ${logicalLines} logical lines, limit ${limit}`);
  }
});
