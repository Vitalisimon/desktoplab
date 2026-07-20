import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { hasUnexpectedSkip } from "./beta-gauntlet-output.mjs";
import { currentRepositoryVisibilityMode } from "./repository-visibility-mode.mjs";
import { waitForSharedProductPorts } from "./shared-port-settle.mjs";

const PROFILES = new Set(["quick", "standard", "full"]);
const MODES = new Set(["internal", "public-export"]);
const startedAt = new Date();
const args = parseArgs(process.argv.slice(2));
const reportPath = args.reportPath ?? path.join("dist", "product", "beta-gauntlet.json");
const steps = buildSteps(args.profile);
const report = {
  schemaVersion: 1,
  profile: args.profile,
  mode: args.mode,
  dryRun: args.dryRun,
  allowDirty: args.allowDirty,
  prebuiltCandidate: args.prebuiltCandidate,
  frontierLocalClaim: args.frontierLocalClaim,
  startedAt: startedAt.toISOString(),
  finishedAt: null,
  status: "running",
  steps: [],
};

console.log(`${gauntletName()} beta gauntlet: ${args.profile}${args.dryRun ? " dry-run" : ""}`);

for (const step of steps) {
  if (step.platforms && !step.platforms.includes(process.platform)) {
    record(step, "skipped", 0, `platform ${process.platform} not in ${step.platforms.join(",")}`);
    continue;
  }
  if (step.requiresPath && !existsSync(step.requiresPath)) {
    record(step, "skipped", 0, `missing ${step.requiresPath}`);
    continue;
  }
  if (args.dryRun) {
    record(step, "planned", 0, commandText(step));
    continue;
  }
  const result = runStep(step);
  if (result.status !== "passed") {
    finish("failed");
    process.exit(result.exitCode || 1);
  }
}

finish("passed");
console.log(`${gauntletName()} beta gauntlet passed: ${args.profile}`);

function buildSteps(profile) {
  const quick = [
    cmd("build-cache-maintenance", "node", ["scripts/product/prune-build-cache.mjs"]),
    cleanTreeStep(),
    cmd("diff-whitespace", "git", ["diff", "--check"]),
    cmd("external-reference-guard", "node", [
      "scripts/product/external-reference-guard.mjs",
      "--mode",
      args.mode,
    ]),
    cmd("claim-guard", "node", ["scripts/product/product-claim-guard.mjs"]),
    cmd("agent-evaluation-boundary", "node", ["scripts/product/agent-evaluation-boundary.mjs"]),
    cmd("client-sdk-contract", "npm", ["--workspace", "@desktoplab/client-sdk", "test"]),
    cmd("surface-audit", "node", ["scripts/product/product-surface-audit.mjs", "--mode", args.mode]),
    ...(args.mode === "internal"
      ? []
      : [cmd("public-export-docs", "node", ["scripts/product/public-export-audit.mjs"])]),
    ...(args.mode === "internal"
      ? [
          cmd("agent-parity-eval", "node", ["scripts/product/agent-parity-eval.mjs"]),
          cmd("complete-local-agent-gate", "node", ["scripts/product/complete-local-agent-gate.mjs"]),
          cmd("live-agent-certification", "node", ["scripts/product/live-agent-certification.mjs"]),
        ]
      : [
          cmd("public-agent-certification-contracts", "node", [
            "--test",
            "scripts/product/agent-trace-score-core.test.mjs",
            "scripts/product/agent-failure-taxonomy.test.mjs",
            "scripts/product/agent-configuration-fingerprint.test.mjs",
            "scripts/product/agent-evaluation-anti-gaming.test.mjs",
            "scripts/product/agent-reliability-campaign.test.mjs",
            "scripts/product/agent-parity-eval.test.mjs",
            "scripts/product/installed-agent-certification.test.mjs",
            "scripts/product/live-agent-certification.test.mjs",
            "scripts/product/stable-ui-certification.test.mjs",
          ]),
        ]),
    ...(args.frontierLocalClaim
      ? [cmd("frontier-local-gate", "node", ["scripts/product/frontier-local-gate.mjs", "--claim", "--certification", args.frontierCertification])]
      : []),
    cmd("operator-control-plane-contracts", "cargo", [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_doctor_lint",
      "--test",
      "local_api_diagnostics_export",
      "--test",
      "local_api_runtime_inspect",
      "--test",
      "local_api_execution_backend_contracts",
      "--test",
      "local_api_stability_diagnostics",
    ]),
    cmd("operator-cli", "cargo", ["test", "-p", "desktoplab-smoke-cli", "--test", "operator_commands"]),
    cmd("agent-parity-routes", "cargo", [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_agent_parity_contract",
      "--test",
      "local_api_agent_file_creation_truth",
      "--test",
      "local_api_agent_tool_approvals",
      "--test",
      "local_api_agent_test_runner",
      "--test",
      "local_api_agent_transcript",
      "--test",
      "local_api_agent_dependency_policy",
      "--test",
      "local_api_agent_multifile_refactor",
      "--test",
      "local_api_agent_memory",
    ]),
    cmd("artifact-budget", "node", ["scripts/product/artifact-budget-guard.mjs"]),
    cmd("frontend-typecheck", "npm", ["--prefix", "apps/desktop", "run", "typecheck"]),
    cmd("frontend-line-guard", "npm", ["--prefix", "apps/desktop", "run", "line-guard"]),
    cmd("catalog-seed", "cargo", ["test", "-p", "desktoplab-compatibility", "--test", "model_seed_catalog"]),
    cmd("model-manager", "cargo", ["test", "-p", "desktoplab-model-manager", "--test", "model_manager", "--test", "model_inventory_service"]),
    cmd("setup-model-routes", "cargo", [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_setup_truth",
      "--test",
      "local_api_model_download",
      "--test",
      "local_api_route_selection",
    ]),
    cmd("security-routes", "cargo", [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_security_boundary",
      "--test",
      "local_api_payload_approvals",
    ]),
    cmd("setup-ui", "npm", [
      "--prefix",
      "apps/desktop",
      "run",
      "test",
      "--",
      "src/features/setup/RecommendationView.test.tsx",
      "src/features/setup/SetupWizard.selection.test.tsx",
      "src/features/productization/AgentWorkspaceFeature.test.tsx",
      "src/features/terminal/TerminalDrawer.test.tsx",
      "src/features/workspaces/RepositoryFileTree.test.tsx",
    ]),
  ];

  const standard = [
    ...quick,
    cmd("backend-product-core", "cargo", [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_first_launch",
      "--test",
      "local_api_setup_completion",
      "--test",
      "local_api_backend_owned_readiness",
      "--test",
      "local_api_agent_truth",
      "--test",
      "local_api_no_product_fallbacks",
    ]),
    cmd("runtime-model-execution", "cargo", [
      "test",
      "-p",
      "desktoplab-runtime",
      "--test",
      "runtime_install_execution",
      "--test",
      "ollama_runtime_operations",
    ]),
    cmd("frontend-unit-suite", "npm", ["--prefix", "apps/desktop", "run", "test"]),
    smoke("fresh-user-flow", "tests/fresh-user-product-flow.spec.ts"),
    smoke("no-route-intercepts", "tests/product-readiness-no-route-intercepts.spec.ts"),
    smoke("setup-to-repository", "tests/product/setup-to-repository.product.spec.ts"),
    smoke("first-prompt", "tests/product/first-prompt.product.spec.ts"),
    smoke("file-drawer", "tests/product/file-drawer.product.spec.ts"),
    smoke("terminal-drawer", "tests/product/terminal-drawer.product.spec.ts"),
    cmd("packaging-provenance", "npm", packagingProvenanceArgs(), {
      requiresPath: "dist/desktoplab-packaging/artifact-manifest.json",
    }),
    cmd("macos-installed-smoke", "bash", ["scripts/packaging/macos-install-smoke.sh", "--dev-artifact", "--app", "/Applications/DesktopLab.app"], {
      platforms: ["darwin"],
      requiresPath: "/Applications/DesktopLab.app",
    }),
    cmd("macos-bundle-metadata", "npm", ["run", "packaging:verify:macos-metadata", "--", "--app", "/Applications/DesktopLab.app"], {
      platforms: ["darwin"],
      requiresPath: "/Applications/DesktopLab.app",
    }),
    cmd("installed-agent-ui-qa", "npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/workbench-visual.spec.ts", "--project=desktop"], {
      platforms: ["darwin"],
      requiresPath: "/Applications/DesktopLab.app",
    }),
  ];

  if (profile === "quick") return [...quick, ...cacheClosureSteps()];
  if (profile === "standard") return [...standard, ...cacheClosureSteps()];
  return [
    ...standard,
    ...(args.prebuiltCandidate
      ? []
      : [
          cmd("desktop-package-dev", "npm", ["run", "desktop:package:dev"]),
          cmd("packaging-provenance-after-build", "npm", packagingProvenanceArgs()),
        ]),
    cmd("product-truth-real", "npm", ["run", "product:truth:real"]),
    cmd("visual-product-audit", "npm", ["--prefix", "apps/desktop", "run", "visual:product-audit"]),
    ...cacheClosureSteps(),
  ];
}

function cacheClosureSteps() {
  return [
    cmd("build-cache-maintenance-final", "node", ["scripts/product/prune-build-cache.mjs"]),
    cmd("artifact-budget-final", "node", ["scripts/product/artifact-budget-guard.mjs"]),
  ];
}

function packagingProvenanceArgs() {
  return process.platform === "darwin"
    ? ["run", "packaging:verify", "--", "--installed-app", "/Applications/DesktopLab.app"]
    : ["run", "packaging:verify"];
}

function cleanTreeStep() {
  return { id: "tracked-clean-tree", kind: "clean-tree" };
}

function cmd(id, command, commandArgs, options = {}) {
  return { id, kind: "command", command, args: commandArgs, ...options };
}

function smoke(id, spec) {
  return cmd(`smoke-${id}`, "npm", ["--prefix", "apps/desktop", "run", "smoke", "--", spec, "--project=desktop"], {
    settleSharedPorts: true,
  });
}

function runStep(step) {
  console.log(`\n[${step.id}] ${commandText(step)}`);
  const started = Date.now();
  if (step.kind === "clean-tree") return runCleanTree(step, started);
  const result = spawnSync(step.command, step.args, {
    cwd: process.cwd(),
    encoding: "utf8",
    env: { ...process.env, CI: process.env.CI ?? "true" },
    maxBuffer: 1024 * 1024 * 64,
  });
  process.stdout.write(result.stdout ?? "");
  process.stderr.write(result.stderr ?? "");
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  const status = result.status === 0 && !hasUnexpectedSkip(output) ? "passed" : "failed";
  const exitCode = result.status ?? 1;
  const note = status === "failed" && hasUnexpectedSkip(output) ? "unexpected skip detected" : commandText(step);
  record(step, status, Date.now() - started, note, exitCode);
  if (step.settleSharedPorts && status === "passed") {
    waitForSharedProductPorts();
  }
  return { status, exitCode };
}

function runCleanTree(step, started) {
  if (args.mode === "public-export" && !isGitCheckout()) {
    record(step, "skipped", Date.now() - started, "public export tree has no git metadata yet");
    return { status: "passed", exitCode: 0 };
  }
  if (args.allowDirty) {
    record(step, "skipped", Date.now() - started, "dirty tree allowed by flag");
    return { status: "passed", exitCode: 0 };
  }
  const result = spawnSync("git", ["status", "--short"], {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  });
  const dirty = (result.stdout ?? "").trim();
  if (result.status !== 0 || dirty.length > 0) {
    if (dirty) console.error(dirty);
    record(step, "failed", Date.now() - started, dirty || "git status failed", result.status ?? 1);
    return { status: "failed", exitCode: result.status || 1 };
  }
  record(step, "passed", Date.now() - started, "tracked tree clean");
  return { status: "passed", exitCode: 0 };
}

function isGitCheckout() {
  const result = spawnSync("git", ["rev-parse", "--is-inside-work-tree"], {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  });
  return result.status === 0 && (result.stdout ?? "").trim() === "true";
}

function record(step, status, durationMs, note, exitCode = 0) {
  report.steps.push({
    id: step.id,
    status,
    durationMs,
    exitCode,
    command: commandText(step),
    note,
  });
}

function finish(status) {
  report.status = status;
  report.finishedAt = new Date().toISOString();
  report.durationMs = new Date(report.finishedAt).getTime() - startedAt.getTime();
  mkdirSync(path.dirname(reportPath), { recursive: true });
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}${os.EOL}`);
  console.log(`Report: ${reportPath}`);
}

function commandText(step) {
  if (step.kind === "clean-tree") return args.allowDirty ? "git status --short (allowed dirty)" : "git status --short";
  return `${step.command} ${step.args.join(" ")}`;
}

function parseArgs(rawArgs) {
  const parsed = {
    profile: "standard",
    mode: currentRepositoryVisibilityMode(),
    dryRun: false,
    allowDirty: false,
    prebuiltCandidate: false,
    frontierLocalClaim: process.env.DESKTOPLAB_FRONTIER_LOCAL_CLAIM === "1",
    frontierCertification: "dist/product/frontier-local-certification.json",
    reportPath: null,
  };
  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = rawArgs[index];
    if (arg === "--dry-run") parsed.dryRun = true;
    else if (arg === "--allow-dirty") parsed.allowDirty = true;
    else if (arg === "--prebuilt-candidate") parsed.prebuiltCandidate = true;
    else if (arg === "--frontier-local-claim") parsed.frontierLocalClaim = true;
    else if (arg === "--frontier-certification") parsed.frontierCertification = rawArgs[++index] ?? "";
    else if (arg === "--profile") parsed.profile = rawArgs[++index] ?? "";
    else if (arg.startsWith("--profile=")) parsed.profile = arg.slice("--profile=".length);
    else if (arg === "--mode") parsed.mode = rawArgs[++index] ?? "";
    else if (arg.startsWith("--mode=")) parsed.mode = arg.slice("--mode=".length);
    else if (arg === "--report") parsed.reportPath = rawArgs[++index] ?? "";
    else if (arg.startsWith("--report=")) parsed.reportPath = arg.slice("--report=".length);
    else if (arg === "--help") usage(0);
    else {
      console.error(`Unknown argument: ${arg}`);
      usage(2);
    }
  }
  if (!PROFILES.has(parsed.profile)) {
    console.error(`Invalid profile: ${parsed.profile}`);
    usage(2);
  }
  if (!MODES.has(parsed.mode)) {
    console.error(`Invalid mode: ${parsed.mode}`);
    usage(2);
  }
  return parsed;
}

function usage(exitCode) {
  console.log(`Usage: node scripts/product/beta-gauntlet.mjs [--profile quick|standard|full] [--mode internal|public-export] [--dry-run] [--allow-dirty] [--prebuilt-candidate] [--frontier-local-claim] [--frontier-certification path] [--report path]`);
  process.exit(exitCode);
}

function gauntletName() {
  return args.mode === "public-export" ? "Public export" : "Private";
}
