#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { verifyAgentReleaseFreeze } from "./agent-release-freeze-core.mjs";

const freezePath = resolve(process.argv[2] ?? "evaluation/release-candidate-agent-freeze.json");
const report = verifyAgentReleaseFreeze(JSON.parse(readFileSync(freezePath, "utf8")), { repoRoot: process.cwd() });
console.log(JSON.stringify(report, null, 2));
process.exitCode = report.status === "pass" ? 0 : 1;
