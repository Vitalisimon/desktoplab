#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

import { validateEvaluationTask } from "./agent-evaluation-anti-gaming-core.mjs";

const privateRoot = ".desktoplab/evaluation/holdouts/";
const failures = [];
const ignored = readFileSync(".gitignore", "utf8").split(/\r?\n/).map((line) => line.trim());
if (!ignored.includes(privateRoot)) failures.push(`${privateRoot} is not explicitly ignored`);

const tracked = execFileSync("git", ["ls-files"], { encoding: "utf8" }).split(/\r?\n/).filter(Boolean);
for (const file of tracked.filter((entry) => entry.startsWith(privateRoot))) failures.push(`${file}: private holdout is tracked`);

const packageInputs = ["apps/desktop/package.json", "apps/desktop/src-tauri/tauri.conf.json", "scripts/product/create-public-export.mjs"];
for (const file of packageInputs) {
  const source = readFileSync(file, "utf8");
  if (source.includes(privateRoot) || source.includes("evaluation/holdouts")) failures.push(`${file}: holdout root enters package/export inputs`);
}

const catalog = JSON.parse(readFileSync("evaluation/development/agent-tasks.json", "utf8"));
if (catalog.kind !== "desktoplab.agent-evaluation-catalog" || catalog.schemaVersion !== 1) failures.push("development catalog contract invalid");
for (const task of catalog.tasks ?? []) {
  if (task.evaluationRole !== "development") failures.push(`${task.taskId}: tracked task must be development-only`);
  failures.push(...validateEvaluationTask(task).map((failure) => `${task.taskId}: ${failure}`));
}

if (failures.length > 0) {
  console.error("Agent evaluation boundary failed:");
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}
console.log(`Agent evaluation boundary passed: ${catalog.tasks.length} development tasks; private holdouts excluded`);
