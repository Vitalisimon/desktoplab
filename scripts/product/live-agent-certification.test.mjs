import test from "node:test";
import assert from "node:assert/strict";

import {
  buildCertificationReport,
  certificationPersonas,
  certificationSurfaces,
  minimumCapabilityCases,
  runCertificationReport,
} from "./live-agent-certification.mjs";

test("deterministic dev certification covers daily agent surfaces across multiple user profiles", () => {
  const report = buildCertificationReport({
    mode: "deterministic-dev",
    env: {},
  });

  assert.equal(report.kind, "desktoplab.live-agent-certification");
  assert.equal(report.status, "pass");
  assert.equal(report.liveClaim, false);
  assert.equal(report.personas.length, 3);
  assert.equal(report.surfaces.length, 12);
  assert.equal(report.cases.length, certificationPersonas.length * certificationSurfaces.length);
  assert.ok(report.surfaces.some((surface) => surface.id === "multi_step_loop"));
  assert.ok(report.surfaces.some((surface) => surface.id === "failure_repair_loop"));
  assert.ok(report.overall >= 0.85, `overall=${report.overall}`);
  assert.deepEqual(report.failures, []);
  assert.ok(report.cases.every((certCase) => certCase.workspaceIsolation === "per_persona_workspace"));
  assert.ok(report.cases.every((certCase) => certCase.sessionIsolation === "per_prompt_thread"));
});

test("certification declares the minimum cases required for full local agent routing", () => {
  const report = buildCertificationReport({
    mode: "deterministic-dev",
    env: {},
  });
  const required = minimumCapabilityCases.map((item) => item.id);

  assert.deepEqual(required, [
    "read",
    "create",
    "patch",
    "test",
    "failure_repair",
    "diff",
    "commit_proposal",
    "refusal",
  ]);
  assert.deepEqual(report.minimumCapabilityCases.map((item) => item.id), required);
  assert.equal(report.localModelCapabilityClass, "deterministic_contract_only");
});

test("live local certification blocks honestly until installed app and model evidence are configured", () => {
  const report = buildCertificationReport({
    mode: "live-local",
    env: {},
  });

  assert.equal(report.status, "blocked_live_requirements");
  assert.equal(report.liveClaim, false);
  assert.equal(report.overall, null);
  assert.deepEqual(report.failures, [
    "missing DESKTOPLAB_LIVE_AGENT_APP",
    "missing DESKTOPLAB_LIVE_AGENT_MODEL",
    "missing DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT",
  ]);
});

test("live local certification can declare readiness only when explicit live evidence exists", () => {
  const report = buildCertificationReport({
    mode: "live-local",
    env: {
      DESKTOPLAB_LIVE_AGENT_APP: "/Applications/DesktopLab.app",
      DESKTOPLAB_LIVE_AGENT_MODEL: "qwen2.5-coder:7b",
      DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT: "/tmp/desktoplab-live-agent-workspaces",
    },
  });

  assert.equal(report.status, "ready_to_run_live");
  assert.equal(report.liveClaim, false);
  assert.equal(report.overall, null);
  assert.equal(report.requirements.appArtifact, "/Applications/DesktopLab.app");
  assert.equal(report.requirements.localModel, "qwen2.5-coder:7b");
  assert.equal(report.requirements.workspaceRoot, "/tmp/desktoplab-live-agent-workspaces");
});

test("live local execution scores every case and can produce a live pass", async () => {
  const prompts = [];
  const report = await runCertificationReport({
    mode: "live-local",
    env: {
      DESKTOPLAB_LIVE_AGENT_APP: "/Applications/DesktopLab.app",
      DESKTOPLAB_LIVE_AGENT_MODEL: "qwen2.5-coder:7b",
      DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT: "/tmp/desktoplab-live-agent-workspaces",
    },
    liveExecutor: async (certCase) => {
      prompts.push(certCase.prompt);
      return JSON.stringify({
        surface: certCase.surfaceId,
        action: `Execute the requested ${certCase.surfaceId} agent action in DesktopLab.`,
        evidence: certCase.evidence[0],
        safety: "Respect approvals, local workspace scope and policy boundaries.",
        transcript: "Record planned, approved, executed and validation events truthfully.",
        validation: "Validate the result from repository evidence before claiming completion.",
      });
    },
  });

  assert.equal(report.status, "pass");
  assert.equal(report.liveClaim, true);
  assert.equal(report.executionKind, "live_local_model");
  assert.equal(report.caseCount, 36);
  assert.equal(prompts.length, 36);
  assert.ok(report.overall >= 0.85, `overall=${report.overall}`);
  assert.ok(report.cases.every((certCase) => certCase.score >= 0.85));
  assert.ok(report.cases.every((certCase) => certCase.liveResponsePreview.includes("\"surface\"")));
});

test("live local execution records model failures instead of aborting the report", async () => {
  const report = await runCertificationReport({
    mode: "live-local",
    env: {
      DESKTOPLAB_LIVE_AGENT_APP: "/Applications/DesktopLab.app",
      DESKTOPLAB_LIVE_AGENT_MODEL: "qwen2.5-coder:7b",
      DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT: "/tmp/desktoplab-live-agent-workspaces",
    },
    liveExecutor: async () => {
      throw new Error("model timeout");
    },
  });

  assert.equal(report.status, "fail");
  assert.equal(report.liveClaim, false);
  assert.equal(report.cases.length, 36);
  assert.ok(report.failures[0].includes("model timeout"));
  assert.ok(report.cases.every((certCase) => certCase.score === 0));
});

test("live local execution accepts expected evidence returned as a JSON array", async () => {
  const report = await runCertificationReport({
    mode: "live-local",
    env: {
      DESKTOPLAB_LIVE_AGENT_APP: "/Applications/DesktopLab.app",
      DESKTOPLAB_LIVE_AGENT_MODEL: "qwen2.5-coder:7b",
      DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT: "/tmp/desktoplab-live-agent-workspaces",
    },
    liveExecutor: async (certCase) =>
      JSON.stringify({
        surface: certCase.surfaceId,
        action: `Execute the requested ${certCase.surfaceId} agent action in DesktopLab.`,
        evidence: certCase.evidence,
        safety: "Respect approvals, local workspace scope and policy boundaries.",
        transcript: "Record planned, approved, executed and validation events truthfully.",
        validation: "Validate the result from repository evidence before claiming completion.",
      }),
  });

  assert.equal(report.status, "pass");
  assert.equal(report.liveClaim, true);
  assert.ok(report.cases.every((certCase) => certCase.score >= 0.85));
});

test("live local execution fails generic prose that does not cite expected evidence", async () => {
  const report = await runCertificationReport({
    mode: "live-local",
    env: {
      DESKTOPLAB_LIVE_AGENT_APP: "/Applications/DesktopLab.app",
      DESKTOPLAB_LIVE_AGENT_MODEL: "qwen2.5-coder:7b",
      DESKTOPLAB_LIVE_AGENT_WORKSPACE_ROOT: "/tmp/desktoplab-live-agent-workspaces",
    },
    liveExecutor: async (certCase) =>
      JSON.stringify({
        surface: certCase.surfaceId,
        action: "I would do the requested work.",
        evidence: "generic evidence",
        safety: "safe",
        transcript: "transcript",
        validation: "validation",
      }),
  });

  assert.equal(report.status, "fail");
  assert.equal(report.liveClaim, false);
  assert.ok(report.overall < 0.85, `overall=${report.overall}`);
  assert.ok(report.failures.some((failure) => failure.includes("live score")));
});
