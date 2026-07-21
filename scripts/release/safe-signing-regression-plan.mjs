import { join, resolve } from "node:path";

export function requiredSafeSigningSteps(inputs, env = process.env) {
  const uiManifest = env.DESKTOPLAB_STABLE_UI_MANIFEST ?? "dist/product/stable-ui-qa/desktop/manifest.json";
  const manualReview = env.DESKTOPLAB_INSTALLED_UI_MANUAL_REVIEW ?? "dist/release/installed-ui-manual-review.json";
  const installedDriver = env.DESKTOPLAB_INSTALLED_AGENT_DRIVER ?? "scripts/product/drivers/macos-installed-agent-ui.mjs";
  const reliabilityUiDriver = "scripts/product/drivers/macos-installed-agent-reliability-ui.mjs";
  const reliabilityVerifier = "scripts/product/recorded-agent-reliability-driver.mjs";
  const uiDriverDependencies = [
    installedDriver,
    "scripts/product/drivers/macos-installed-agent-ui-wait.mjs",
    "scripts/product/drivers/macos-installed-agent-reliability-run.mjs",
    "scripts/product/drivers/reliability-run-collector.mjs",
    "scripts/product/drivers/memory-pressure-helper.mjs",
  ];
  return [
    command("clean-tree", "git", ["status", "--porcelain=v1"], { rejectOutput: true }),
    command("candidate-payload", "node", ["scripts/release/candidate-admission.mjs", "verify", "--candidate", inputs.candidate, "--app", inputs.app]),
    command("rust-workspace", "cargo", ["test", "--locked", "--workspace"]),
    command("tauri-tests", "cargo", ["test", "--locked", "--manifest-path", "apps/desktop/src-tauri/Cargo.toml"]),
    command("frontend-typecheck", "npm", ["--prefix", "apps/desktop", "run", "typecheck"]),
    command("frontend-tests", "npm", ["--prefix", "apps/desktop", "run", "test"]),
    command("frontend-line-guard", "npm", ["--prefix", "apps/desktop", "run", "line-guard"]),
    command("product-truth", "npm", ["run", "product:truth:real"]),
    command("installed-agent", "node", ["scripts/product/installed-agent-certification.mjs", "--app", inputs.app, "--workspace", inputs.workspace, "--evidence", inputs.evidence, "--candidate", inputs.candidate, "--driver", installedDriver, "--report", inputs.certification], { timeoutMs: 90 * 60 * 1000 }),
    command("measured-agent-runtime", "node", ["scripts/product/agent-parity-eval.mjs", "--evidence", inputs.certification, "--json", "--report", inputs.runtime]),
    command("agent-reliability-recording", "node", [reliabilityUiDriver, "--app", inputs.app, "--candidate", inputs.candidate, "--output-root", inputs.reliabilityRoot, "--manifest", inputs.reliabilityManifest, "--catalog", inputs.reliabilityCatalog], { timeoutMs: 4 * 60 * 60 * 1000 }),
    command("agent-reliability-campaign", "node", ["scripts/product/agent-reliability-campaign.mjs", "--manifest", inputs.reliabilityManifest, "--driver", reliabilityVerifier, "--report", inputs.campaign], { env: { DESKTOPLAB_RELIABILITY_CATALOG: inputs.reliabilityCatalog }, timeoutMs: 90 * 60 * 1000 }),
    command("agent-release-gates", "node", ["scripts/release/agent-release-gates.mjs", "--candidate", inputs.candidate, "--runtime", inputs.runtime, "--campaign", inputs.campaign, "--executor", reliabilityVerifier, "--ui-driver", reliabilityUiDriver, ...uiDriverDependencies.flatMap((path) => ["--ui-driver-dependency", path]), "--output", inputs.agentGates]),
    command("beta-quick", "node", ["scripts/product/beta-gauntlet.mjs", "--profile", "quick", "--report", "dist/release/beta-quick.json"]),
    command("beta-full", "node", ["scripts/product/beta-gauntlet.mjs", "--profile", "full", "--prebuilt-candidate", "--report", "dist/release/beta-full.json"]),
    command("stable-ui", "node", ["scripts/product/stable-ui-certification.mjs", "--manifest", uiManifest, "--manual-review", manualReview, "--candidate", inputs.candidate, "--app", inputs.app]),
    command("npm-advisories", "npm", ["audit", "--omit=dev", "--audit-level=high"]),
    command("cargo-advisories", "cargo", ["audit"]),
    command("tracked-secret-scan", "node", ["scripts/security/scan-tracked-secrets.mjs"]),
  ];
}

export function candidateInputs(parsed, runId) {
  const runRoot = resolve(parsed.runRoot ?? `dist/release/candidate/regression-runs/${safeRunId(runId)}`);
  const reliabilityRoot = resolve(parsed.reliabilityRoot ?? join(runRoot, "reliability-recording"));
  return {
    candidate: resolve(parsed.candidate ?? "dist/release/candidate/admission.json"),
    app: resolve(parsed.app ?? "/Applications/DesktopLab.app"),
    workspace: resolve(parsed.workspace ?? join(runRoot, "installed-agent-workspace")),
    evidence: resolve(parsed.evidence ?? join(runRoot, "installed-agent-evidence.json")),
    certification: resolve(parsed.certification ?? join(runRoot, "installed-agent-certification.json")),
    runtime: resolve(parsed.runtime ?? join(runRoot, "measured-agent-parity.json")),
    campaign: resolve(parsed.campaign ?? join(runRoot, "agent-reliability-campaign.json")),
    agentGates: resolve(parsed.agentGates ?? join(runRoot, "agent-release-gates.json")),
    reliabilityRoot,
    reliabilityManifest: resolve(parsed.reliabilityManifest ?? join(reliabilityRoot, "manifest.json")),
    reliabilityCatalog: resolve(parsed.reliabilityCatalog ?? join(reliabilityRoot, "catalog.json")),
  };
}

export function safeRunId(value) {
  return value.replace(/[^a-zA-Z0-9._-]/g, "_");
}

function command(id, executable, args, options = {}) {
  return { id, command: executable, args, required: true, ...options };
}
