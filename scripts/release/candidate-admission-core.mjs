import { createHash } from "node:crypto";

export const candidateStates = [
  "source_admitted",
  "payload_built",
  "pre_sign_pass",
  "signed",
  "post_sign_pass",
  "cross_platform_pass",
  "draft_ready",
  "rejected",
];

const releaseSequence = candidateStates.filter((state) => state !== "rejected");

export function admitCandidateSource({ source, version, channel, lockfiles, createdAt }) {
  assert(source?.status === "pass", "canonical public source gate did not pass");
  assert(/^[a-f0-9]{40}$/.test(source.head ?? ""), "candidate source commit is invalid");
  assert(source.treeState === "clean", "candidate source tree is not clean");
  assert(/^\d+\.\d+\.\d+$/.test(version ?? ""), "candidate version is invalid");
  assert(["beta", "stable"].includes(channel), "candidate channel is invalid");
  assertLockfiles(lockfiles);

  const identity = {
    repository: source.canonicalRepository,
    commit: source.head,
    version,
    channel,
    lockfiles,
  };
  const candidateId = `sha256:${digest(identity)}`;
  return {
    kind: "desktoplab.release-candidate",
    schemaVersion: 1,
    candidateId,
    state: "source_admitted",
    source: {
      repository: source.canonicalRepository,
      origin: source.origin,
      commit: source.head,
      treeState: source.treeState,
    },
    release: { version, channel },
    lockfiles,
    payload: null,
    transitions: [],
    createdAt: createdAt ?? new Date().toISOString(),
  };
}

export function bindCandidatePayload(candidate, payload) {
  assertCandidate(candidate);
  assert(candidate.state === "source_admitted", "candidate payload can only bind after source admission");
  assert(payload?.platform === "macos-aarch64", "candidate payload platform is invalid");
  assert(typeof payload.relativePath === "string" && payload.relativePath.length > 0, "candidate payload path is missing");
  assert(/^[a-f0-9]{64}$/.test(payload.sha256 ?? ""), "candidate payload hash is invalid");
  assert(payload.sizeBytes > 0, "candidate payload size is invalid");
  return {
    ...candidate,
    state: "payload_built",
    payload: { ...payload },
    transitions: [...(candidate.transitions ?? []), transitionRecord("payload_built", { kind: "desktoplab.prepared-payload", sha256: payload.sha256 })],
  };
}

export function transitionCandidate(candidate, { to, evidence, transitionedAt } = {}) {
  assertCandidate(candidate);
  assert(candidate.state !== "rejected" && candidate.state !== "draft_ready", "candidate is already terminal");
  const expected = releaseSequence[releaseSequence.indexOf(candidate.state) + 1];
  assert(to === expected, `candidate transition ${candidate.state} -> ${to} is not allowed; expected ${expected}`);
  assertTransitionEvidence(candidate, to, evidence);
  return {
    ...candidate,
    state: to,
    transitions: [...(candidate.transitions ?? []), transitionRecord(to, evidence, transitionedAt)],
  };
}

export function rejectCandidate(candidate, { reason, evidence = null, transitionedAt } = {}) {
  assertCandidate(candidate);
  assert(!["rejected", "draft_ready"].includes(candidate.state), "candidate is already terminal");
  assert(typeof reason === "string" && reason.trim().length > 0, "candidate rejection reason is required");
  return {
    ...candidate,
    state: "rejected",
    rejection: { reason: reason.trim() },
    transitions: [...(candidate.transitions ?? []), transitionRecord("rejected", evidence ?? { kind: "desktoplab.operator-rejection", reason }, transitionedAt)],
  };
}

export function verifyCandidate({ candidate, source, lockfiles, payload }) {
  const failures = [];
  try {
    assertCandidate(candidate);
  } catch (error) {
    failures.push(error.message);
  }
  if (source?.status !== "pass") failures.push("canonical public source gate did not pass");
  if (candidate?.source?.commit !== source?.head) failures.push("candidate source commit differs from current public HEAD");
  if (candidate?.source?.repository !== source?.canonicalRepository) failures.push("candidate repository differs from canonical public source");
  if (JSON.stringify(candidate?.lockfiles) !== JSON.stringify(lockfiles)) failures.push("candidate lock hashes differ from current source");
  if (payload) {
    if (candidate?.state === "source_admitted") failures.push("candidate has no bound payload");
    if (candidate?.payload?.sha256 !== payload.sha256 || candidate?.payload?.sizeBytes !== payload.sizeBytes) {
      failures.push("candidate payload mutated after admission");
    }
  }
  return {
    kind: "desktoplab.release-candidate-verification",
    schemaVersion: 1,
    status: failures.length === 0 ? "pass" : "fail",
    candidateId: candidate?.candidateId ?? null,
    state: candidate?.state ?? null,
    failures,
  };
}

export function assertCandidate(candidate) {
  assert(candidate?.kind === "desktoplab.release-candidate", "candidate kind is invalid");
  assert(candidate.schemaVersion === 1, "candidate schema version is invalid");
  assert(/^sha256:[a-f0-9]{64}$/.test(candidate.candidateId ?? ""), "candidate id is invalid");
  assert(candidateStates.includes(candidate.state), "candidate state is invalid");
  const identity = {
    repository: candidate.source?.repository,
    commit: candidate.source?.commit,
    version: candidate.release?.version,
    channel: candidate.release?.channel,
    lockfiles: candidate.lockfiles,
  };
  assert(candidate.candidateId === `sha256:${digest(identity)}`, "candidate identity was mutated");
}

function assertLockfiles(lockfiles) {
  assert(Array.isArray(lockfiles) && lockfiles.length > 0, "candidate lock hashes are missing");
  for (const lock of lockfiles) {
    assert(typeof lock.path === "string" && /^[a-f0-9]{64}$/.test(lock.sha256 ?? ""), "candidate lock hash is invalid");
  }
}

function assertTransitionEvidence(candidate, to, evidence) {
  assert(evidence && typeof evidence === "object", `candidate transition ${to} requires evidence`);
  if (to === "pre_sign_pass") {
    const run = evidence.runs?.at?.(-1);
    assert(evidence.kind === "desktoplab.safe-signing-regression" && run?.status === "pass", "pre-sign transition requires passing safe-signing evidence");
    assert(run.candidateId === candidate.candidateId, "safe-signing evidence belongs to another candidate");
    assert(run.preparedAppSha256 === candidate.payload?.sha256, "safe-signing evidence belongs to another payload");
  } else if (to === "signed") {
    assert(evidence.kind === "desktoplab.artifact-provenance" && evidence.schemaVersion === 2, "signed transition requires artifact provenance");
    assert(evidence.build?.commitSha === candidate.source.commit, "signed artifact commit differs from candidate");
    assert(evidence.entries?.filter((entry) => ["app_bundle", "distribution_file"].includes(entry.kind)).every((entry) => entry.signatureState === "notarized"), "signed artifact evidence is not fully notarized");
  } else if (to === "post_sign_pass") {
    assert(evidence.kind === "desktoplab.installed-agent-certification" && evidence.schemaVersion === 3 && evidence.status === "pass", "post-sign transition requires passing installed-agent certification");
    assert(evidence.provenance?.candidateId === candidate.candidateId, "post-sign certification belongs to another candidate");
  } else if (to === "cross_platform_pass") {
    assert(evidence.kind === "desktoplab.platform-candidate-convergence" && evidence.status === "pass", "cross-platform transition requires passing convergence evidence");
    assert(evidence.commit === candidate.source.commit, "cross-platform evidence differs from candidate commit");
  } else if (to === "draft_ready") {
    assert(evidence.kind === "desktoplab.release-assembly" && evidence.status === "draft-ready", "draft transition requires release assembly evidence");
    assert(evidence.source?.commit === candidate.source.commit, "release assembly differs from candidate commit");
  }
}

function transitionRecord(state, evidence, transitionedAt) {
  return {
    state,
    evidenceKind: evidence?.kind ?? null,
    evidenceSha256: digest(evidence ?? null),
    transitionedAt: transitionedAt ?? new Date().toISOString(),
  };
}

function digest(value) {
  return createHash("sha256").update(JSON.stringify(value)).digest("hex");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
