import { spawnSync } from "node:child_process";
import { waitForSharedProductPorts } from "./shared-port-settle.mjs";

const commands = [
  ["cargo", ["test", "-p", "xtask", "--test", "product_truth_gate"]],
  [
    "cargo",
    [
      "test",
      "-p",
      "desktoplab-control-plane",
      "--test",
      "local_api_first_launch",
      "--test",
      "local_api_setup_completion",
      "--test",
      "local_api_setup_truth",
      "--test",
      "local_api_backend_owned_readiness",
      "--test",
      "local_api_runtime_execution",
      "--test",
      "local_api_model_execution",
      "--test",
      "local_api_agent_truth",
      "--test",
      "first_prompt_product_step",
      "--test",
      "workbench_restart_state",
      "--test",
      "provider_api_key_vault",
      "--test",
      "provider_security_policy",
      "--test",
      "local_api_provider_vault",
      "--test",
      "local_api_auth_boundary",
      "--test",
      "local_api_origin_boundary",
      "--test",
      "local_api_discovery",
      "--test",
      "local_api_discovery_permissions",
      "--test",
      "local_api_security_boundary",
    ],
  ],
  ["cargo", ["test", "-p", "desktoplab-agent-engine", "--test", "local_inference_boundary", "--test", "openai_compatible_endpoint", "--test", "workspace_context", "--test", "worktree_write_policy"]],
  ["cargo", ["test", "-p", "desktoplab-runtime", "--test", "runtime_install_execution"]],
  ["cargo", ["test", "-p", "desktoplab-model-manager", "--test", "model_download_execution_live_contract"]],
  ["cargo", ["test", "--manifest-path", "apps/desktop/src-tauri/Cargo.toml", "--test", "packaged_boot_truth"]],
  ["npm", ["run", "product:claim-guard"]],
  ["npm", ["--prefix", "apps/desktop", "run", "typecheck"]],
  ["npm", ["--prefix", "apps/desktop", "run", "line-guard"]],
  ["npm", ["--prefix", "apps/desktop", "run", "test"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product-readiness-no-route-intercepts.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/setup-to-repository.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/runtime-install.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/model-download.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/first-prompt.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/file-drawer.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/terminal-drawer.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/approval-flow.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/workbench-empty-states.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/workbench-restart.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/provider-truth.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/model-readiness-restart.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/setup-pipeline-restart.product.spec.ts", "--project=desktop"]],
  ["npm", ["--prefix", "apps/desktop", "run", "smoke", "--", "tests/product/setup-accessibility.product.spec.ts", "--project=desktop"]],
];

for (const [command, args] of commands) {
  run(command, args);
}

console.log("Product truth regression passed");

function run(command, args) {
  console.log(`\n$ ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  process.stdout.write(result.stdout ?? "");
  process.stderr.write(result.stderr ?? "");
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
  if (unexpectedSkip(output)) {
    console.error("Product truth regression failed: unexpected skipped product proof.");
    process.exit(1);
  }
  if (command === "npm" && args.includes("smoke")) {
    waitForSharedProductPorts();
  }
}

function unexpectedSkip(output) {
  return /\b(?:skipped|skip)\b/i.test(output) && !/narrow skipped by design/i.test(output);
}
