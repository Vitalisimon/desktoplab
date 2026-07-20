import { execFileSync } from "node:child_process";
import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import {
  boundedError,
  buildSnapshot,
  collectConnection,
  evaluateRequirements,
  flattenRestPages,
  publicSummary,
} from "./github-support-audit-core.mjs";

const API_VERSION = "2026-03-10";
const DISCUSSIONS_QUERY = `
  query($owner: String!, $name: String!, $after: String) {
    repository(owner: $owner, name: $name) {
      discussions(first: 100, after: $after, orderBy: {field: UPDATED_AT, direction: DESC}) {
        nodes {
          id number title body url createdAt updatedAt closed closedAt locked activeLockReason
          author { login }
          authorAssociation
          category { id name slug description emoji isAnswerable }
          answer { id }
          answerChosenAt
          answerChosenBy { login }
          labels(first: 100) { nodes { name } }
        }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
`;
const DISCUSSION_COMMENTS_QUERY = `
  query($id: ID!, $after: String) {
    node(id: $id) {
      ... on Discussion {
        comments(first: 100, after: $after) {
          nodes {
            id body url createdAt updatedAt isAnswer upvoteCount
            author { login }
            authorAssociation
          }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }
`;
const DISCUSSION_REPLIES_QUERY = `
  query($id: ID!, $after: String) {
    node(id: $id) {
      ... on DiscussionComment {
        replies(first: 100, after: $after) {
          nodes {
            id body url createdAt updatedAt upvoteCount
            author { login }
            authorAssociation
          }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }
`;

export class GitHubCliClient {
  constructor({ maxBuffer = 64 * 1024 * 1024 } = {}) {
    this.maxBuffer = maxBuffer;
  }

  run(args) {
    return JSON.parse(execFileSync("gh", args, {
      encoding: "utf8",
      maxBuffer: this.maxBuffer,
      stdio: ["ignore", "pipe", "pipe"],
    }));
  }

  rest(path) {
    return this.run([
      "api", "--method", "GET",
      "-H", "Accept: application/vnd.github+json",
      "-H", `X-GitHub-Api-Version: ${API_VERSION}`,
      path,
    ]);
  }

  restAll(path) {
    return flattenRestPages(this.run([
      "api", "--method", "GET", "--paginate", "--slurp",
      "-H", "Accept: application/vnd.github+json",
      "-H", `X-GitHub-Api-Version: ${API_VERSION}`,
      path,
    ]));
  }

  graphql(query, variables) {
    const args = ["api", "graphql", "-f", `query=${query}`];
    for (const [key, value] of Object.entries(variables)) {
      if (value !== null && value !== undefined) args.push("-F", `${key}=${value}`);
    }
    return this.run(args).data;
  }
}

export async function collectIssues(client, repo) {
  const records = client.restAll(`repos/${repo}/issues?state=all&per_page=100`)
    .filter((issue) => !issue.pull_request);
  const issues = [];
  for (const issue of records) {
    const comments = client.restAll(`repos/${repo}/issues/${issue.number}/comments?per_page=100`);
    const timeline = client.restAll(`repos/${repo}/issues/${issue.number}/timeline?per_page=100`);
    issues.push({ ...issue, comments, timeline, auditCoverage: { comments: "complete", timeline: "complete" } });
  }
  return issues;
}

export async function collectDiscussions(client, owner, name) {
  const discussions = await collectConnection((after) => {
    return client.graphql(DISCUSSIONS_QUERY, { owner, name, after }).repository.discussions;
  });
  const output = [];
  for (const discussion of discussions.nodes) {
    const comments = await collectConnection((after) => {
      return client.graphql(DISCUSSION_COMMENTS_QUERY, { id: discussion.id, after }).node.comments;
    });
    const completeComments = [];
    for (const comment of comments.nodes) {
      const replies = await collectConnection((after) => {
        return client.graphql(DISCUSSION_REPLIES_QUERY, { id: comment.id, after }).node.replies;
      });
      completeComments.push({ ...comment, replies: replies.nodes, auditCoverage: { replies: "complete", pages: replies.pages } });
    }
    output.push({
      ...discussion,
      comments: completeComments,
      auditCoverage: { comments: "complete", commentPages: comments.pages },
    });
  }
  return output;
}

export function collectAdvisories(client, repo) {
  const summaries = client.restAll(`repos/${repo}/security-advisories?per_page=100`);
  return summaries.map((advisory) => client.rest(`repos/${repo}/security-advisories/${advisory.ghsa_id}`));
}

async function collectChannel(name, operation) {
  try {
    return { records: await operation(), access: { state: "complete" } };
  } catch (error) {
    return { records: [], access: { state: "unavailable", reason: boundedError(error), channel: name } };
  }
}

function detectRepoSlug(explicit) {
  if (explicit) return explicit;
  const remote = execFileSync("git", ["remote", "get-url", "origin"], { encoding: "utf8" }).trim();
  const match = remote.match(/github\.com[/:]([^/]+\/[^/.]+?)(?:\.git)?$/);
  if (!match) throw new Error("GITHUB_REPOSITORY_NOT_DETECTED");
  return match[1];
}

async function main() {
  const { values } = parseArgs({
    options: {
      repo: { type: "string" },
      output: { type: "string", default: "dist/support-audit/github-support-snapshot.json" },
      require: { type: "string", default: "issues,discussions" },
    },
  });
  const repo = detectRepoSlug(values.repo);
  const [owner, name] = repo.split("/");
  if (!owner || !name) throw new Error("INVALID_REPOSITORY_SLUG");

  const client = new GitHubCliClient();
  const repository = client.rest(`repos/${repo}`);
  let privateVulnerabilityReporting = "unavailable";
  try {
    const state = client.rest(`repos/${repo}/private-vulnerability-reporting`);
    privateVulnerabilityReporting = state.enabled ? "enabled" : "disabled";
  } catch {
    privateVulnerabilityReporting = repository.visibility === "public" ? "unavailable" : "requires-public-repository";
  }

  const issues = await collectChannel("issues", () => collectIssues(client, repo));
  const discussions = await collectChannel("discussions", () => collectDiscussions(client, owner, name));
  const advisories = await collectChannel("advisories", () => collectAdvisories(client, repo));
  if (repository.visibility !== "public" && advisories.access.state === "unavailable") {
    advisories.access = { state: "requires-public-repository" };
  }
  const sourceCommit = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  const snapshot = buildSnapshot({
    repository,
    sourceCommit,
    access: {
      issues: issues.access,
      discussions: discussions.access,
      advisories: advisories.access,
      privateVulnerabilityReporting,
    },
    issues: issues.records,
    discussions: discussions.records,
    advisories: advisories.records,
  });

  const outputPath = resolve(values.output);
  mkdirSync(dirname(outputPath), { recursive: true, mode: 0o700 });
  chmodSync(dirname(outputPath), 0o700);
  writeFileSync(outputPath, `${JSON.stringify(snapshot, null, 2)}\n`, { mode: 0o600 });
  chmodSync(outputPath, 0o600);

  const required = values.require.split(",").map((value) => value.trim()).filter(Boolean);
  const findings = evaluateRequirements(snapshot, required);
  console.log(JSON.stringify(publicSummary(snapshot, outputPath, findings), null, 2));
  if (findings.length > 0) process.exitCode = 1;
}

const isMain = process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url));
if (isMain) main().catch((error) => {
  console.error(`GitHub support audit failed: ${boundedError(error)}`);
  process.exitCode = 1;
});
