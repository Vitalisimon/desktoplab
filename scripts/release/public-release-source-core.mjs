const DEFAULT_CANONICAL_REPOSITORY = "github.com/vitalisimon/desktoplab";

export function assessPublicReleaseSource({
  origin,
  head,
  publishedHead,
  treeState,
  trackedPaths = [],
  canonicalRepository = DEFAULT_CANONICAL_REPOSITORY,
} = {}) {
  const failures = [];
  const normalizedOrigin = normalizeRepositoryOrigin(origin);
  if (normalizedOrigin !== canonicalRepository.toLowerCase()) {
    failures.push(`release source origin is not canonical public repository: ${normalizedOrigin || "missing"}`);
  }
  if (!/^[a-f0-9]{40}$/.test(head ?? "")) failures.push("release source HEAD is missing or invalid");
  if (!/^[a-f0-9]{40}$/.test(publishedHead ?? "")) {
    failures.push("canonical public main HEAD is unavailable");
  } else if (publishedHead !== head) {
    failures.push("release source HEAD is not the published canonical main HEAD");
  }
  if (treeState !== "clean") failures.push("release source tree is not clean");

  const privatePaths = trackedPaths.filter(isPrivateReleasePath);
  if (privatePaths.length > 0) {
    failures.push(`release source tracks private-only paths: ${privatePaths.join(", ")}`);
  }

  return {
    kind: "desktoplab.public-release-source",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    canonicalRepository,
    origin: normalizedOrigin,
    head: head ?? null,
    publishedHead: publishedHead ?? null,
    treeState: treeState ?? null,
    privatePaths,
    failures,
  };
}

export function normalizeRepositoryOrigin(origin) {
  const value = String(origin ?? "").trim();
  if (!value) return "";
  const ssh = value.match(/^git@([^:]+):(.+)$/);
  if (ssh) return `${ssh[1]}/${stripGitSuffix(ssh[2])}`.toLowerCase();
  try {
    const url = new URL(value);
    return `${url.hostname}/${stripGitSuffix(url.pathname.replace(/^\//, ""))}`.toLowerCase();
  } catch {
    return stripGitSuffix(value).toLowerCase();
  }
}

function stripGitSuffix(value) {
  return value.replace(/\.git\/?$/, "").replace(/\/$/, "");
}

export function isPrivateReleasePath(path) {
  return path === "AGENTS.md"
    || path === "PUBLIC_EXPORT_MANIFEST.json"
    || path === ".env"
    || path.startsWith(".env.")
    || path.startsWith("docs/");
}
