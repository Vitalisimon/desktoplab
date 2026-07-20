const SHA_PATTERN = /^[0-9a-f]{40}$/;
const ALLOWED_DECISIONS = new Set(["adapt", "already-covered", "defer", "reject"]);

export function assessExternalReferencePolicy({
  ignoreSource,
  trackedFiles,
  manifestSources,
  ledger,
  requireLedger = true,
}) {
  const failures = [];

  if (!ignoreSource.split(/\r?\n/).some((line) => line.trim() === ".external-references/")) {
    failures.push(".gitignore must ignore .external-references/");
  }

  for (const file of trackedFiles) {
    if (file === ".external-references" || file.startsWith(".external-references/")) {
      failures.push(`${file}: external reference checkouts must not be tracked`);
    }
  }

  for (const { path, source } of manifestSources) {
    for (const pattern of [/github\.com\/openclaw\//i, /git@github\.com:openclaw\//i, /@openclaw\//i]) {
      if (pattern.test(source)) {
        failures.push(`${path}: OpenClaw dependency or build input is not allowed`);
        break;
      }
    }
  }

  if (!ledger && !requireLedger) return failures;
  if (!ledger || ledger.schemaVersion !== 1 || !Array.isArray(ledger.references)) {
    failures.push("reference ledger must use schemaVersion 1 with a references array");
    return failures;
  }

  const remotes = new Set();
  for (const [index, reference] of ledger.references.entries()) {
    const label = reference?.remote || `references[${index}]`;
    if (!/^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\.git$/.test(reference?.remote ?? "")) {
      failures.push(`${label}: remote must be a canonical GitHub HTTPS clone URL`);
    }
    if (remotes.has(reference?.remote)) failures.push(`${label}: duplicate remote`);
    remotes.add(reference?.remote);
    if (!SHA_PATTERN.test(reference?.commit ?? "")) failures.push(`${label}: commit must be a full lowercase SHA-1`);
    if (reference?.licenseObserved !== "MIT") failures.push(`${label}: license observation must be explicit`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(reference?.observedAt ?? "")) {
      failures.push(`${label}: observedAt must be YYYY-MM-DD`);
    }
    if (!ALLOWED_DECISIONS.has(reference?.decision)) failures.push(`${label}: invalid DesktopLab decision`);
    if (typeof reference?.learningScope !== "string" || reference.learningScope.trim().length < 8) {
      failures.push(`${label}: learningScope is missing or too vague`);
    }
  }

  if (ledger.references.length < 18) failures.push("reference ledger must include the primary repo and all reviewed satellites");
  return failures;
}
