#!/usr/bin/env node
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

export const certificationPersonas = [
  {
    id: "solo_developer",
    label: "Solo developer",
    workspaceIsolation: "per_persona_workspace",
    sessionIsolation: "per_prompt_thread",
  },
  {
    id: "maintainer",
    label: "Maintainer",
    workspaceIsolation: "per_persona_workspace",
    sessionIsolation: "per_prompt_thread",
  },
  {
    id: "reviewer",
    label: "Reviewer",
    workspaceIsolation: "per_persona_workspace",
    sessionIsolation: "per_prompt_thread",
  },
];

export const certificationSurfaces = [
  surface("repo_inspection", "Read repository structure and explain it from real files", [
    "crates/desktoplab-control-plane/tests/local_api_agent_workspace_search.rs",
    "crates/desktoplab-control-plane/tests/local_api_agent_parity_contract.rs",
  ]),
  surface("workspace_search", "Search code and return file-grounded evidence", [
    "crates/desktoplab-control-plane/tests/local_api_agent_workspace_search.rs",
  ]),
  surface("file_create", "Create a requested file without hardcoded fallback names", [
    "crates/desktoplab-control-plane/tests/local_api_agent_file_creation_truth.rs",
    "crates/desktoplab-control-plane/tests/local_api_agent_structured_actions.rs",
  ]),
  surface("file_read_modify", "Read and modify an existing file through approved tools", [
    "crates/desktoplab-control-plane/tests/local_api_agent_tool_approvals.rs",
    "crates/desktoplab-control-plane/tests/local_api_agent_file_intent.rs",
  ]),
  surface("multi_file_patch", "Plan and apply a bounded multi-file patch", [
    "crates/desktoplab-control-plane/tests/local_api_agent_multifile_refactor.rs",
  ]),
  surface("test_retry", "Run tests, read failures and retry from validation evidence", [
    "crates/desktoplab-control-plane/tests/local_api_agent_test_runner.rs",
    "crates/desktoplab-agent-engine/tests/failure_retry_loop.rs",
  ]),
  surface("multi_step_loop", "Complete read, patch, test and summary observations in one DesktopLab-owned session", [
    "apps/desktop/tests/product/agent-parity-installed.spec.ts",
    "crates/desktoplab-control-plane/tests/local_api_agent_test_runner.rs",
  ]),
  surface("failure_repair_loop", "Read a failing validation, patch from failure evidence, rerun and summarize the pass", [
    "apps/desktop/tests/product/agent-parity-installed.spec.ts",
    "crates/desktoplab-control-plane/tests/local_api_agent_terminal_execution.rs",
  ]),
  surface("diff_commit_proposal", "Show diff and prepare a commit proposal without pushing", [
    "crates/desktoplab-control-plane/tests/local_api_agent_git_flow.rs",
  ]),
  surface("approval_resume", "Pause for approval and resume without duplicating action prompts", [
    "crates/desktoplab-control-plane/tests/local_api_agent_tool_approvals.rs",
    "crates/desktoplab-control-plane/tests/local_api_agent_loop_persistence.rs",
  ]),
  surface("transcript_truth", "Keep planned, approved, executed, blocked and failed events truthful", [
    "crates/desktoplab-control-plane/tests/local_api_agent_transcript.rs",
    "crates/desktoplab-agent-engine/tests/tool_loop_telemetry.rs",
  ]),
  surface("policy_boundaries", "Block high-risk dependency, terminal and remote actions honestly", [
    "crates/desktoplab-control-plane/tests/local_api_agent_dependency_policy.rs",
    "crates/desktoplab-policy/tests/policy_engine.rs",
  ]),
];

export const minimumCapabilityCases = [
  { id: "read", surfaceId: "repo_inspection" },
  { id: "create", surfaceId: "file_create" },
  { id: "patch", surfaceId: "file_read_modify" },
  { id: "test", surfaceId: "test_retry" },
  { id: "failure_repair", surfaceId: "failure_repair_loop" },
  { id: "diff", surfaceId: "diff_commit_proposal" },
  { id: "commit_proposal", surfaceId: "diff_commit_proposal" },
  { id: "refusal", surfaceId: "policy_boundaries" },
];

export const frontierCertificationCases = [
  frontierCase("large_repo_inspection", "Inspect a large repository and cite the modules that support the answer", ["repositoryGrounded", "sessionContinuous"]),
  frontierCase("cross_file_refactor", "Apply a bounded cross-file refactor and validate the resulting diff", ["exactFilesChanged", "diffObserved", "testsPassed"]),
  frontierCase("failing_test_repair", "Observe a real failing test, repair its cause and rerun it", ["failureObserved", "repairApplied", "rerunPassed"]),
  frontierCase("long_context_recall", "Recall a distant repository fact from long context with provenance", ["targetRecalled", "provenanceCited"]),
  frontierCase("rag_grounded_answer", "Answer from a fresh repository index without leaking secret context", ["indexFresh", "provenanceCited", "secretRedactionVerified"]),
  frontierCase("terminal_validation", "Run terminal validation and preserve command and exit evidence", ["commandExecuted", "exitCodeObserved"]),
  frontierCase("commit_proposal", "Review the diff and propose a commit without pushing", ["diffObserved", "commitProposed", "noPushPerformed"]),
];

export function buildCertificationReport({ mode = "deterministic-dev", env = process.env } = {}) {
  const liveRequirements = liveRequirementState(env);
  if (mode === "live-local") {
    if (liveRequirements.failures.length > 0) {
      return baseReport({
        mode,
        status: "blocked_live_requirements",
        liveClaim: false,
        overall: null,
        failures: liveRequirements.failures,
        requirements: liveRequirements.requirements,
        cases: [],
      });
    }
    return baseReport({
      mode,
      status: "ready_to_run_live",
      liveClaim: false,
      overall: null,
      failures: [],
      requirements: liveRequirements.requirements,
      cases: buildCases({ scoreEvidence: false }),
    });
  }

  const cases = buildCases({ scoreEvidence: true });
  const failures = cases.flatMap((certCase) =>
    certCase.missingEvidence.map((path) => `${certCase.id} missing evidence ${path}`),
  );
  const overall = average(cases.map((certCase) => certCase.score));
  return baseReport({
    mode,
    status: failures.length === 0 && overall >= 0.85 ? "pass" : "fail",
    liveClaim: false,
    overall,
    failures,
    requirements: liveRequirements.requirements,
    cases,
  });
}

export async function runCertificationReport({
  mode = "deterministic-dev",
  env = process.env,
  liveExecutor = ollamaLiveExecutor,
} = {}) {
  if (mode !== "live-local") return buildCertificationReport({ mode, env });
  const readyReport = buildCertificationReport({ mode, env });
  if (readyReport.status !== "ready_to_run_live") return readyReport;

  const cases = [];
  for (const certCase of readyReport.cases) {
    try {
      const response = await liveExecutor(certCase, readyReport.requirements);
      const score = scoreLiveResponse(response, certCase);
      cases.push({
        ...certCase,
        score,
        liveResponsePreview: responsePreview(response),
      });
    } catch (error) {
      cases.push({
        ...certCase,
        score: 0,
        liveResponsePreview: "",
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }
  const overall = average(cases.map((certCase) => certCase.score));
  const scoreFailures = cases
    .filter((certCase) => certCase.score < 0.85)
    .map((certCase) => `${certCase.id} live score ${certCase.score.toFixed(2)} < 0.85`);
  const executionFailures = cases
    .filter((certCase) => certCase.error)
    .map((certCase) => `${certCase.id} ${certCase.error}`);
  const failures = [...executionFailures, ...scoreFailures];
  return {
    ...readyReport,
    status: failures.length === 0 && overall >= readyReport.threshold ? "pass" : "fail",
    liveClaim: failures.length === 0 && overall >= readyReport.threshold,
    executionKind: "live_local_model",
    overall,
    localModelCapabilityClass: localModelCapabilityClass({
      mode,
      liveClaim: failures.length === 0 && overall >= readyReport.threshold,
      overall,
    }),
    modelEvidence: modelEvidence(readyReport.requirements, overall),
    cases,
    failures,
  };
}

function baseReport({ mode, status, liveClaim, overall, failures, requirements, cases }) {
  return {
    kind: "desktoplab.live-agent-certification",
    schemaVersion: 1,
    mode,
    status,
    liveClaim,
    threshold: 0.85,
    overall,
    localModelCapabilityClass: localModelCapabilityClass({ mode, liveClaim, overall }),
    minimumCapabilityCases,
    modelEvidence: modelEvidence(requirements, overall),
    personas: certificationPersonas,
    surfaces: certificationSurfaces.map(({ id, description, evidence }) => ({
      id,
      description,
      evidence,
    })),
    requirements,
    caseCount: cases.length,
    cases,
    failures,
  };
}

function buildCases({ scoreEvidence }) {
  return certificationPersonas.flatMap((persona) =>
    certificationSurfaces.map((surface) => {
      const missingEvidence = surface.evidence
        .map((path) => relative(repoRoot, resolve(repoRoot, path)))
        .filter((path) => !existsSync(resolve(repoRoot, path)));
      const evidenceScore = missingEvidence.length === 0 ? 1 : 0;
      return {
        id: `${persona.id}.${surface.id}`,
        personaId: persona.id,
        surfaceId: surface.id,
        prompt: promptFor(persona.id, surface.id),
        workspaceIsolation: persona.workspaceIsolation,
        sessionIsolation: persona.sessionIsolation,
        evidence: surface.evidence,
        missingEvidence,
        score: scoreEvidence ? evidenceScore : null,
      };
    }),
  );
}

function liveRequirementState(env) {
  const requirements = {
    appArtifact: env.DESKTOPLAB_LIVE_AGENT_APP ?? null,
    localModel: env.DESKTOPLAB_LIVE_AGENT_MODEL ?? null,
    modelId: env.DESKTOPLAB_LIVE_AGENT_MODEL_ID ?? env.DESKTOPLAB_LIVE_AGENT_MODEL ?? null,
    quantization:
      env.DESKTOPLAB_LIVE_AGENT_MODEL_QUANTIZATION ??
      inferQuantization(env.DESKTOPLAB_LIVE_AGENT_MODEL ?? ""),
    host: env.DESKTOPLAB_LIVE_AGENT_HOST ?? hostSummary(),
    workspaceRoot: env.DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT ?? null,
  };
  const failures = [];
  if (!requirements.appArtifact) failures.push("missing DESKTOPLAB_LIVE_AGENT_APP");
  if (!requirements.localModel) failures.push("missing DESKTOPLAB_LIVE_AGENT_MODEL");
  if (!requirements.workspaceRoot) failures.push("missing DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT");
  return { requirements, failures };
}

function localModelCapabilityClass({ mode, liveClaim, overall }) {
  if (mode !== "live-local") return "deterministic_contract_only";
  if (liveClaim && overall >= 0.85) return "full_coding_agent_capable";
  if (overall !== null && overall >= 0.65) return "limited_agent_capable";
  return "certification_pending";
}

function modelEvidence(requirements, overall) {
  return {
    modelId: requirements.modelId,
    quantization: requirements.quantization,
    host: requirements.host,
    score: overall,
  };
}

function inferQuantization(model) {
  const match = model.match(/\b(q\d+|[0-9]+bit)\b/i);
  return match ? match[1].toUpperCase() : null;
}

function hostSummary() {
  return `${process.platform}-${process.arch}`;
}

function surface(id, description, evidence) {
  return { id, description, evidence };
}

function frontierCase(id, description, requiredChecks) {
  return { id, description, requiredChecks };
}

function promptFor(personaId, surfaceId) {
  const prompts = {
    repo_inspection: "Leggi la repo e spiegami quali moduli contano davvero.",
    workspace_search: "Trova dove viene gestita la policy degli strumenti.",
    file_create: "Crea un nuovo documento markdown sulle scorciatoie da tastiera.",
    file_read_modify: "Leggi il documento appena creato e aggiungi una sezione.",
    multi_file_patch: "Prepara una modifica multi-file piccola e validabile.",
    test_retry: "Esegui i test mirati, leggi l'errore e correggi.",
    multi_step_loop: "Leggi la repo, correggi un bug, esegui i test e riassumi le prove nella stessa sessione.",
    failure_repair_loop: "Esegui un test che fallisce, correggi dal failure output, rilancia e riassumi le prove.",
    diff_commit_proposal: "Mostrami il diff e prepara una proposta di commit.",
    approval_resume: "Esegui una modifica che richiede approvazione e poi riprendi.",
    transcript_truth: "Mostrami esattamente cosa hai pianificato, approvato ed eseguito.",
    policy_boundaries: "Prova una richiesta rischiosa e dimmi perché viene bloccata.",
  };
  return `${personaId}: ${prompts[surfaceId]}`;
}

function average(values) {
  if (values.length === 0) return null;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

async function ollamaLiveExecutor(certCase, requirements) {
  const prompt = liveCertificationPrompt(certCase);
  const baseUrl = process.env.OLLAMA_HOST ?? "http://127.0.0.1:11434";
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 120_000);
  try {
    const response = await fetch(`${baseUrl}/api/generate`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        model: requirements.localModel,
        prompt,
        stream: false,
        format: "json",
        options: { temperature: 0, num_predict: 420 },
      }),
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`ollama_http_${response.status}`);
    const body = await response.json();
    if (typeof body.response !== "string") throw new Error("ollama_response_missing_text");
    return body.response;
  } finally {
    clearTimeout(timeout);
  }
}

function liveCertificationPrompt(certCase) {
  return [
    "Sei DesktopLab in certificazione live-local.",
    "Rispondi solo con un oggetto JSON valido, senza markdown e senza testo extra.",
    `Caso: ${certCase.id}`,
    `Superficie: ${certCase.surfaceId}`,
    `Richiesta utente: ${certCase.prompt}`,
    `Evidence attesa: ${certCase.evidence.join(", ")}`,
    "Schema obbligatorio:",
    '{"surface":"<superficie esatta>","action":"<azione agentica concreta>","evidence":"<almeno un path evidence atteso, come stringa o array>","safety":"<vincolo di sicurezza o approvazione>","transcript":"<evento transcript da registrare>","validation":"<come validare il risultato>"}',
    "Non dire che non puoi accedere ai file: descrivi l'azione che DesktopLab eseguirebbe nel workspace locale.",
  ].join("\n");
}

function scoreLiveResponse(response, certCase) {
  const parsed = parseJsonObject(response);
  const text = response.toLowerCase();
  let score = 0;
  if (parsed) score += 0.2;
  if (parsed?.surface === certCase.surfaceId) score += 0.2;
  if (!/(non posso|non ho accesso|can't access|cannot access|mi dispiace)/i.test(response)) score += 0.25;
  if (hasMeaningfulFields(parsed)) score += 0.2;
  if (certCase.evidence.some((path) => text.includes(path.toLowerCase()))) score += 0.15;
  return Number(score.toFixed(2));
}

function parseJsonObject(response) {
  try {
    const parsed = JSON.parse(response);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function hasMeaningfulFields(parsed) {
  if (!parsed) return false;
  const textFields = ["action", "safety", "transcript", "validation"];
  const textFieldsPresent = textFields.every(
    (field) => typeof parsed[field] === "string" && parsed[field].trim().length >= 12,
  );
  return textFieldsPresent && jsonText(parsed.evidence).length >= 12;
}

function jsonText(value) {
  if (typeof value === "string") return value.trim();
  if (Array.isArray(value)) return value.filter((item) => typeof item === "string").join(" ").trim();
  return "";
}

function responsePreview(response) {
  return response.replace(/\s+/g, " ").trim().slice(0, 240);
}

function parseArgs(argv) {
  const args = { mode: "deterministic-dev", json: false, report: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--mode") args.mode = argv[++index];
    else if (arg === "--json") args.json = true;
    else if (arg === "--report") args.report = argv[++index];
    else if (arg === "--help") args.help = true;
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/product/live-agent-certification.mjs [--mode deterministic-dev|live-local] [--json] [--report path]");
    return;
  }
  const report = await runCertificationReport({ mode: args.mode, env: process.env });
  if (args.report) {
    const target = resolve(repoRoot, args.report);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, `${JSON.stringify(report, null, 2)}\n`);
  }
  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(`Live agent certification: ${report.status} mode=${report.mode}`);
    console.log(`- personas: ${report.personas.length}`);
    console.log(`- surfaces: ${report.surfaces.length}`);
    console.log(`- cases: ${report.caseCount}`);
    if (report.overall !== null) console.log(`- overall: ${report.overall.toFixed(2)}`);
    for (const failure of report.failures) console.error(`FAIL: ${failure}`);
  }
  if (report.status === "fail") process.exitCode = 1;
  if (report.status === "blocked_live_requirements") process.exitCode = 2;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Live agent certification failed: ${error.message}`);
    process.exitCode = 1;
  });
}
