export async function collectConnection(fetchPage, { maxPages = 10_000 } = {}) {
  const nodes = [];
  const seenCursors = new Set();
  let after = null;
  let pages = 0;

  while (true) {
    if (pages >= maxPages) throw new Error("PAGINATION_LIMIT_EXCEEDED");
    const connection = await fetchPage(after);
    if (!connection || !Array.isArray(connection.nodes) || !connection.pageInfo) {
      throw new Error("INVALID_CONNECTION_PAGE");
    }

    nodes.push(...connection.nodes);
    pages += 1;
    if (!connection.pageInfo.hasNextPage) return { nodes, pages, complete: true };

    const cursor = connection.pageInfo.endCursor;
    if (!cursor || seenCursors.has(cursor)) throw new Error("INVALID_PAGINATION_CURSOR");
    seenCursors.add(cursor);
    after = cursor;
  }
}

export function flattenRestPages(value) {
  if (!Array.isArray(value)) throw new Error("INVALID_REST_PAGINATION_RESPONSE");
  if (value.length === 0) return [];
  if (value.every(Array.isArray)) return value.flat();
  return value;
}

export function buildSnapshot({ repository, sourceCommit, access, issues, discussions, advisories }) {
  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    sourceCommit,
    repository: {
      fullName: repository.full_name,
      url: repository.html_url,
      visibility: repository.visibility,
      defaultBranch: repository.default_branch,
      features: {
        issues: repository.has_issues,
        discussions: repository.has_discussions,
        privateVulnerabilityReporting: access.privateVulnerabilityReporting,
      },
    },
    coverage: {
      issues: access.issues,
      discussions: access.discussions,
      advisories: access.advisories,
    },
    counts: {
      issues: issues.length,
      issueComments: issues.reduce((total, issue) => total + issue.comments.length, 0),
      issueTimelineEvents: issues.reduce((total, issue) => total + issue.timeline.length, 0),
      discussions: discussions.length,
      discussionComments: discussions.reduce((total, discussion) => total + discussion.comments.length, 0),
      discussionReplies: discussions.reduce(
        (total, discussion) => total + discussion.comments.reduce((count, comment) => count + comment.replies.length, 0),
        0,
      ),
      advisories: advisories.length,
    },
    issues,
    discussions,
    advisories,
  };
}

export function evaluateRequirements(snapshot, requiredChannels) {
  const findings = [];
  const required = new Set(requiredChannels);
  if (required.has("issues") && !snapshot.repository.features.issues) {
    findings.push("issues:disabled");
  } else if (required.has("issues") && snapshot.coverage.issues.state !== "complete") {
    findings.push(`issues:${snapshot.coverage.issues.state}`);
  }
  if (required.has("discussions") && !snapshot.repository.features.discussions) {
    findings.push("discussions:disabled");
  } else if (required.has("discussions") && snapshot.coverage.discussions.state !== "complete") {
    findings.push(`discussions:${snapshot.coverage.discussions.state}`);
  }
  if (required.has("advisories") && snapshot.coverage.advisories.state !== "complete") {
    findings.push(`advisories:${snapshot.coverage.advisories.state}`);
  }
  if (required.has("pvr") && snapshot.repository.features.privateVulnerabilityReporting !== "enabled") {
    findings.push(`pvr:${snapshot.repository.features.privateVulnerabilityReporting}`);
  }
  return findings;
}

export function publicSummary(snapshot, outputPath, findings) {
  return {
    repository: snapshot.repository.fullName,
    visibility: snapshot.repository.visibility,
    outputPath,
    mode: "local-confidential-snapshot",
    permissions: "0600",
    counts: snapshot.counts,
    coverage: snapshot.coverage,
    privateVulnerabilityReporting: snapshot.repository.features.privateVulnerabilityReporting,
    findings,
  };
}

export function boundedError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return message.replace(/\s+/g, " ").trim().slice(0, 500) || "UNKNOWN_ERROR";
}
