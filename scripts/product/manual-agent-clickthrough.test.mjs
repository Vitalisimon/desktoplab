import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  buildManualClickthroughReport,
  manualClickthroughCases,
  manualEvidenceTemplate,
} from "./manual-agent-clickthrough.mjs";

test("manual clickthrough exposes installed-app prompts before evidence exists", () => {
  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    exists: (path) => path === "/Applications/DesktopLab.app",
  });

  assert.equal(report.kind, "desktoplab.manual-agent-clickthrough");
  assert.equal(report.status, "manual_required");
  assert.equal(report.manualClaim, false);
  assert.equal(report.caseCount, 6);
  assert.equal(report.cases.length, 6);
  assert.ok(report.cases.some((certCase) => certCase.requiresApproval));
  assert.ok(report.cases.every((certCase) => certCase.manualSteps.includes("Click Send prompt.")));
});

test("manual clickthrough includes existing-file patch certification evidence", () => {
  const patchCase = manualClickthroughCases.find((certCase) => certCase.id === "existing_file_patch");

  assert.ok(patchCase);
  assert.equal(patchCase.prompt, "Modifica notes.md aggiornando beta in beta updated.");
  assert.equal(patchCase.requiresApproval, true);
  assert.equal(patchCase.expectedApprovalDecision, "approve");
  assert.deepEqual(patchCase.setupFiles, [{ path: "notes.md", content: "alpha\nbeta\ngamma\n" }]);
  assert.equal(patchCase.expectedFileAfter.path, "notes.md");
  assert.equal(patchCase.expectedFileAfter.content, "alpha\nbeta updated\ngamma\n");
  assert.equal(patchCase.requiresDiffEvidence, true);
});

test("manual clickthrough blocks honestly when the installed app is missing", () => {
  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    exists: () => false,
  });

  assert.equal(report.status, "blocked_app_missing");
  assert.equal(report.manualClaim, false);
  assert.deepEqual(report.failures, ["missing app /Applications/DesktopLab.app"]);
  assert.equal(report.caseCount, 0);
});

test("manual clickthrough passes only with complete click evidence for every prompt", () => {
  const observations = manualClickthroughCases.map((certCase) => ({
    id: certCase.id,
    promptSent: true,
    sendClicked: true,
    outputObserved: true,
    transcriptContinuityObserved: true,
    expectedObservationMet: true,
    approvalClicked: certCase.requiresApproval,
    approvalDecision: certCase.expectedApprovalDecision ?? "not_required",
    beforeContent: certCase.expectedFileAfter ? "alpha\nbeta\ngamma\n" : "",
    afterContent: certCase.expectedFileAfter ? certCase.expectedFileAfter.content : "",
    approvalId: certCase.expectedFileAfter ? "approval.patch.notes" : "",
    visibleDiffObserved: certCase.requiresDiffEvidence === true,
    notes: "Observed in installed app manual clickthrough.",
  }));

  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    evidencePath: "manual-evidence.json",
    exists: (path) => path === "/Applications/DesktopLab.app" || path === "manual-evidence.json",
    readFile: () => JSON.stringify({ observations }),
  });

  assert.equal(report.status, "pass");
  assert.equal(report.manualClaim, true);
  assert.equal(report.overall, 1);
  assert.deepEqual(report.failures, []);
});

test("manual clickthrough fails existing-file patch when concrete patch evidence is missing", () => {
  const observations = manualClickthroughCases.map((certCase) => ({
    id: certCase.id,
    promptSent: true,
    sendClicked: true,
    outputObserved: true,
    transcriptContinuityObserved: true,
    expectedObservationMet: true,
    approvalClicked: certCase.requiresApproval,
    approvalDecision: certCase.expectedApprovalDecision ?? "not_required",
    beforeContent: "",
    afterContent: "",
    approvalId: "",
    visibleDiffObserved: false,
  }));

  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    evidencePath: "manual-evidence.json",
    exists: (path) => path === "/Applications/DesktopLab.app" || path === "manual-evidence.json",
    readFile: () => JSON.stringify({ observations }),
  });

  assert.equal(report.status, "fail");
  assert.equal(report.manualClaim, false);
  assert.ok(report.failures.some((failure) => failure.includes("existing_file_patch")));
});

test("manual clickthrough fails partial evidence instead of inferring success", () => {
  const observations = manualClickthroughCases.map((certCase, index) => ({
    id: certCase.id,
    promptSent: true,
    sendClicked: index !== 2,
    outputObserved: true,
    transcriptContinuityObserved: true,
    expectedObservationMet: true,
    approvalClicked: certCase.requiresApproval,
    approvalDecision: certCase.expectedApprovalDecision ?? "not_required",
  }));

  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    evidencePath: "manual-evidence.json",
    exists: (path) => path === "/Applications/DesktopLab.app" || path === "manual-evidence.json",
    readFile: () => JSON.stringify({ observations }),
  });

  assert.equal(report.status, "fail");
  assert.equal(report.manualClaim, false);
  assert.ok(report.failures.some((failure) => failure.includes("file_read_modify")));
});

test("manual clickthrough fails click evidence when the expected observation was not met", () => {
  const observations = manualClickthroughCases.map((certCase) => ({
    id: certCase.id,
    promptSent: true,
    sendClicked: true,
    outputObserved: true,
    transcriptContinuityObserved: true,
    expectedObservationMet: certCase.id !== "repo_inspection",
    approvalClicked: certCase.requiresApproval,
    approvalDecision: certCase.expectedApprovalDecision ?? "not_required",
  }));

  const report = buildManualClickthroughReport({
    appPath: "/Applications/DesktopLab.app",
    evidencePath: "manual-evidence.json",
    exists: (path) => path === "/Applications/DesktopLab.app" || path === "manual-evidence.json",
    readFile: () => JSON.stringify({ observations }),
  });

  assert.equal(report.status, "fail");
  assert.equal(report.manualClaim, false);
  assert.ok(report.failures.some((failure) => failure.includes("repo_inspection")));
});

test("manual evidence template contains an observation row for every case", () => {
  const template = manualEvidenceTemplate();

  assert.equal(template.observations.length, 6);
  assert.deepEqual(
    template.observations.map((observation) => observation.id),
    manualClickthroughCases.map((certCase) => certCase.id),
  );
  assert.ok(template.observations.find((observation) => observation.id === "existing_file_patch").hasOwnProperty("afterContent"));
});

test("manual clickthrough runner stays below the line guard", () => {
  const source = readFileSync(new URL("./manual-agent-clickthrough.mjs", import.meta.url), "utf8");

  assert.ok(source.split("\n").length <= 220);
});
