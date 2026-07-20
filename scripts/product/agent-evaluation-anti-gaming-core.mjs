import { createHash } from "node:crypto";

const digestPattern = /^sha256:[a-f0-9]{64}$/i;
const protectedDefault = ".desktoplab/evaluation/holdouts";

export function prepareEvaluationTask(task, seed) {
  const failures = validateEvaluationTask(task);
  if (failures.length > 0) return { status: "blocked", executionTask: null, evaluator: null, failures };
  const prompt = materialize(task, seed);
  return {
    status: "pass",
    executionTask: {
      kind: "desktoplab.agent-evaluation-task",
      schemaVersion: 1,
      taskId: task.taskId,
      capability: task.capability,
      evaluationRole: task.evaluationRole,
      promptStyle: task.promptStyle,
      prompt,
      verifier: { kind: task.verifier.kind, digest: task.verifier.digest.toLowerCase() },
      variantDigest: sha256(`${task.taskId}\0${seed}\0${prompt}`),
    },
    evaluator: {
      protectedPaths: [...new Set([protectedDefault, ...(task.evaluator?.protectedPaths ?? [])])],
      canaryTokens: [...new Set(task.evaluator?.canaryTokens ?? [])],
    },
    failures: [],
  };
}

export function assessEvaluationRun(prepared, result) {
  if (prepared?.status !== "pass") {
    return { status: "blocked", classification: "invalid_evaluation_task", failures: prepared?.failures ?? ["evaluation task missing"] };
  }
  const failures = [];
  const evidence = JSON.stringify({ output: result?.assistantOutput ?? "", trace: result?.trace ?? [] });
  const accessedProtectedPath = prepared.evaluator.protectedPaths.some((path) => normalized(evidence).includes(normalized(path)));
  const echoedCanary = prepared.evaluator.canaryTokens.some((token) => token.length >= 12 && evidence.includes(token));
  const verifierAccess = /(?:read|open|cat)[^\n]{0,120}(?:holdout|verifier|expected[_ -]?output)/i.test(evidence);
  if (accessedProtectedPath || verifierAccess) failures.push("verifier file access detected");
  if (echoedCanary || result?.expectedOutputEchoed === true) failures.push("expected-output echo detected");
  if (result?.verification?.status !== "pass" || !digestPattern.test(result?.verification?.evidenceDigest ?? "")) {
    failures.push("deterministic verifier did not pass");
  }
  const gaming = accessedProtectedPath || verifierAccess || echoedCanary || result?.expectedOutputEchoed === true;
  return {
    status: failures.length === 0 ? "pass" : "fail",
    classification: gaming ? "suspected_verifier_gaming" : failures.length > 0 ? "capability_failure" : null,
    claimEligible: failures.length === 0 && prepared.executionTask.evaluationRole === "holdout",
    failures,
  };
}

export function validateEvaluationTask(task) {
  const failures = [];
  if (task?.evaluationRole !== "development" && task?.evaluationRole !== "holdout") failures.push("evaluationRole invalid");
  if (!safeText(task?.taskId) || !safeText(task?.capability)) failures.push("task identity invalid");
  if (task?.promptStyle !== "natural_vague") failures.push("promptStyle must be natural_vague");
  if (!Array.isArray(task?.promptTemplates) || task.promptTemplates.length < 2) failures.push("at least two prompt templates required");
  for (const prompt of task?.promptTemplates ?? []) {
    if (!safeText(prompt) || /verifier|expected output|sha256|exact contents?/i.test(prompt)) failures.push("prompt template exposes evaluator detail");
  }
  if (!task?.variables || Object.keys(task.variables).length === 0) failures.push("randomized variables required");
  for (const [name, values] of Object.entries(task?.variables ?? {})) {
    if (!safeText(name) || !Array.isArray(values) || values.length < 2 || values.some((value) => !safeText(value))) failures.push(`variable ${name} invalid`);
  }
  if (!safeText(task?.verifier?.kind) || !digestPattern.test(task?.verifier?.digest ?? "")) failures.push("deterministic verifier invalid");
  if (task?.evaluator?.canaryTokens?.some((token) => typeof token !== "string" || token.length < 12)) failures.push("canary token invalid");
  return [...new Set(failures)];
}

export function developmentOnlyClaimViolation(line) {
  const development = /development(?:-only)?|public fixture|public benchmark/i.test(line);
  const completeClaim = /(?:proves?|certif(?:y|ies|ied)|demonstrates?)[^\n]{0,100}(?:complete|full|codex[- /]?class|claude[- /]?class)[^\n]{0,60}agent/i.test(line)
    || /(?:complete|full|codex[- /]?class|claude[- /]?class)[^\n]{0,60}agent[^\n]{0,100}(?:proven|certified|ready)/i.test(line);
  return development && completeClaim;
}

function materialize(task, seed) {
  const random = seededRandom(`${task.taskId}\0${seed}`);
  let prompt = choose(task.promptTemplates, random);
  for (const [name, values] of Object.entries(task.variables).sort(([left], [right]) => left.localeCompare(right))) {
    prompt = prompt.replaceAll(`{${name}}`, choose(values, random));
  }
  if (/\{[^}]+\}/.test(prompt)) throw new Error("unresolved evaluation variable");
  return prompt;
}

function choose(values, random) {
  return values[Math.floor(random() * values.length)];
}

function seededRandom(seed) {
  let state = Number.parseInt(createHash("sha256").update(seed).digest("hex").slice(0, 8), 16);
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function normalized(value) {
  return value.replaceAll("\\", "/").toLowerCase();
}

function safeText(value) {
  return typeof value === "string" && value.trim().length > 0 && value.length <= 500;
}

function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
