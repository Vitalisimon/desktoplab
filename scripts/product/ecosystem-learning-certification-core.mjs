const OUTCOMES = new Set(["implemented", "equivalent", "rejected", "deferred"]);
const PRIORITIES = new Set(["P0", "P1", "P2"]);

export const REQUIRED_TASKS = Array.from({ length: 27 }, (_, index) => 140 + index);
export const REQUIRED_SURFACES = [
  "trace-reliability",
  "acp-sdk-workflows",
  "filesystem-durable-state",
  "remote-lab-adapters",
  "plugins-registry-mcp",
  "operator-visual-support",
];

export function assessEcosystemLearningCertification(input) {
  const failures = [];
  validatePlan(input.planSource, failures);
  validateAdoptionLedger(input.adoptionLedger, failures);
  validateReferenceLedger(input.referenceLedger, failures);
  validateNoCopyBoundary(input, failures);
  validateEvidence(input.evidencePaths, failures);
  return failures;
}

function validatePlan(source = "", failures) {
  for (const task of REQUIRED_TASKS) {
    const start = source.indexOf(`### Task 24.9.${task} -`);
    const next = source.indexOf("### Task 24.9.", start + 1);
    const section = start < 0 ? "" : source.slice(start, next < 0 ? undefined : next);
    if (!/^Status: (implemented|completed)$/m.test(section)) {
      failures.push(`task 24.9.${task} is not implemented or completed`);
    }
  }
}

function validateAdoptionLedger(ledger, failures) {
  if (ledger?.schemaVersion !== 1 || !Array.isArray(ledger?.tasks)) {
    failures.push("adoption ledger must use schemaVersion 1 with a tasks array");
    return;
  }
  const tasks = new Map(ledger.tasks.map((entry) => [entry.task, entry]));
  for (const task of REQUIRED_TASKS) {
    const entry = tasks.get(`24.9.${task}`);
    if (!entry) {
      failures.push(`adoption ledger is missing task 24.9.${task}`);
      continue;
    }
    if (!OUTCOMES.has(entry.outcome)) failures.push(`${entry.task}: invalid outcome`);
    if (!meaningful(entry.owner)) failures.push(`${entry.task}: owner is required`);
    if (!Array.isArray(entry.evidence) || entry.evidence.length === 0) {
      failures.push(`${entry.task}: executable evidence is required`);
    }
  }
  if (tasks.size !== REQUIRED_TASKS.length) failures.push("adoption ledger contains an unexpected task set");
  validatePersonas(ledger.personas, failures);
  for (const risk of ledger.residualRisks ?? []) {
    if (!PRIORITIES.has(risk.priority)) failures.push(`${risk.id}: invalid residual-risk priority`);
    if (!meaningful(risk.owner) || !meaningful(risk.exitGate)) failures.push(`${risk.id}: owner and exit gate are required`);
  }
  for (const finding of ledger.openFindings ?? []) {
    if (["P0", "P1"].includes(finding.priority) && !meaningful(finding.owner)) {
      failures.push(`${finding.id}: ${finding.priority} finding has no owner`);
    }
  }
}

function validatePersonas(personas, failures) {
  if (!Array.isArray(personas) || personas.length !== 6) {
    failures.push("exactly six deterministic audit personas are required");
    return;
  }
  const ids = new Set();
  const covered = new Set();
  for (const persona of personas) {
    if (!meaningful(persona.id) || ids.has(persona.id)) failures.push("audit persona ids must be unique");
    ids.add(persona.id);
    if (!meaningful(persona.owner)) failures.push(`${persona.id}: audit owner is required`);
    for (const surface of persona.surfaces ?? []) {
      if (!REQUIRED_SURFACES.includes(surface)) failures.push(`${persona.id}: unknown audit surface ${surface}`);
      covered.add(surface);
    }
    if (!Array.isArray(persona.checks) || persona.checks.length === 0) failures.push(`${persona.id}: checks are required`);
  }
  for (const surface of REQUIRED_SURFACES) {
    if (!covered.has(surface)) failures.push(`audit surface is not covered: ${surface}`);
  }
}

function validateReferenceLedger(ledger, failures) {
  if (ledger?.schemaVersion !== 1 || !Array.isArray(ledger?.references) || ledger.references.length < 18) {
    failures.push("reference ledger must retain at least 18 pinned references");
    return;
  }
  for (const reference of ledger.references) {
    if (!/^[0-9a-f]{40}$/.test(reference.commit ?? "")) failures.push(`${reference.remote}: reference commit is not pinned`);
  }
}

function validateNoCopyBoundary(input, failures) {
  for (const path of input.trackedFiles ?? []) {
    if (path === ".external-references" || path.startsWith(".external-references/")) {
      failures.push(`${path}: reference checkout is tracked`);
    }
    if (path === "dist" || path.startsWith("dist/")) failures.push(`${path}: generated artifact is tracked`);
  }
  for (const { path, source } of input.dependencySources ?? []) {
    if (/github\.com[/:]openclaw\//i.test(source) || /["']@openclaw\//i.test(source)) {
      failures.push(`${path}: OpenClaw dependency or build input detected`);
    }
  }
  for (const path of input.artifactPaths ?? []) {
    if (/(^|[/\\])(?:openclaw|@openclaw)(?:[/\\]|$)/i.test(path)) {
      failures.push(`${path}: reference identity entered generated artifacts`);
    }
  }
}

function validateEvidence(paths = [], failures) {
  const available = new Set(paths);
  for (const required of [
    "docs/evidence/openclaw-ecosystem-reference-ledger.json",
    "docs/evidence/openclaw-ecosystem-adoption-ledger.json",
    "docs/evidence/cross-platform-agent-parity.md",
    "docs/evidence/filesystem-race-audit.md",
    "docs/evidence/remote-target-contract.md",
  ]) {
    if (!available.has(required)) failures.push(`missing certification evidence ${required}`);
  }
}

function meaningful(value) {
  return typeof value === "string" && value.trim().length >= 3;
}
