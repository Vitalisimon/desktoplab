import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { developmentOnlyClaimViolation } from "./agent-evaluation-anti-gaming-core.mjs";

const guardedPaths = [
  "docs-public",
  "docs/packaging-release-candidate-gate.md",
  "docs/public-beta-readiness-gate.md",
  "docs/product-truth-freeze.md",
  "apps/desktop/src/features",
  "apps/desktop/src/design",
];

const forbidden = [
  /runtime install(?:ation)? is (?:ready|complete|supported)/i,
  /model download is (?:ready|complete|supported)/i,
  /first prompt is (?:ready|complete|supported)/i,
  /public beta (?:is )?(?:ready|available|accepted)/i,
  /packaging release candidate gate\s*\n\s*status:\s*accepted/i,
  /raw tokens never enter DesktopLab/i,
  /Stored in system vault/i,
  /External providers, pushes and protected data still stop for you\./i,
  /Full access.*(?:bypass|ignores|skips).*(?:approval|policy|security)/i,
  /provider egress.*automatic/i,
  /Claude Agent SDK.*is publicly supported/i,
  /ACP.*is publicly supported/i,
  /MCP tool bridge.*is publicly supported/i,
  /MCP.*(?:invocation|runtime).*\b(ready|supported|available|certified|executable)\b/i,
  /cross-platform coding-agent parity.*(?:ready|supported|available)/i,
  /complete local agent.*(?:ready|supported|available|accepted)/i,
  /\b(?:custom\s+)?RAG\b[^\n]{0,120}\b(?:equals|matches|reaches|delivers|provides)\b[^\n]{0,120}\bfrontier(?:[- ]model)?\b/i,
  /\bfrontier(?:[- ]model)?\b[^\n]{0,120}\b(?:through|because of|from|with)\b[^\n]{0,60}\b(?:custom\s+)?RAG\b/i,
  /\bChanged\s+\$\{?message\.slice/i,
  /\bChanged\s+\$\{?[^}\n]+filesystem\.write/i,
];

const guardedClaimLines = [
  {
    label: "frontier-local capability claim",
    pattern:
      /\b(frontier[- ]local capable|frontier[- ]equivalent|DGX[- ]equivalent|DGX(?: Station| Spark)?[^.]*\b(?:ready|supported|certified|available))\b/i,
  },
  {
    label: "runtime support claim",
    pattern: /\b(Ollama|LM Studio|llama\.cpp|MLX|vLLM)\b.*\b(ready|supported|complete|available)\b/i,
  },
  {
    label: "provider support claim",
    pattern:
      /\b(OpenAI|Anthropic|Gemini|OpenRouter|Codex|Claude|ACP|Custom OpenAI-compatible endpoint)\b.*\b(ready|supported|available|executable)\b/i,
  },
  {
    label: "agent protocol support claim",
    pattern: /\b(Claude Agent SDK|Claude app bridge|ACP|MCP|external agent bridge|Codex bridge)\b.*\b(ready|supported|available|executable|certified)\b/i,
  },
  {
    label: "descriptor metadata support claim",
    pattern:
      /\b(?:descriptor|inspect|metadata)(?:[- ]only)?\b.*\b(?:plugin|runtime|backend)\b.*\b(?:ready|supported|available|certified|executable)\b/i,
  },
  {
    label: "descriptor metadata support claim",
    pattern:
      /\b(?:plugin|runtime|backend)\b.*\b(?:descriptor|inspect|metadata)(?:[- ]only)?\b.*\b(?:ready|supported|available|certified|executable)\b/i,
  },
  {
    label: "setup/product readiness claim",
    pattern: /\b(setup|first launch|first prompt|workbench)\b.*\b(ready|complete|supported|available)\b/i,
  },
];

const allowedTruthQualifiers =
  /\b(Not publicly claimed|not publicly|not claimed|unclaimed|private-dev|PRIVATE-DEV-CONTRACT|not advertised|not available|not executable|not supported|future|blocked|unsupported|architected|detected|configured|requires|certification|required before support|draft|guard|not imply|superseded|only after|only until)\b/i;

const files = guardedPaths.flatMap((entry) => collectGuardedFiles(entry));
const failures = [];

for (const file of files) {
  if (isIgnoredSource(file)) continue;
  const source = readFileSync(file, "utf8");
  for (const pattern of forbidden) {
    if (pattern.test(source)) {
      failures.push(`${file}: ${pattern}`);
    }
  }
  source.split("\n").forEach((line, index) => {
    if (isIgnoredLine(line)) return;
    if (developmentOnlyClaimViolation(line)) {
      failures.push(`${file}:${index + 1}: development-only evidence cannot certify a complete agent`);
    }
    for (const claim of guardedClaimLines) {
      if (claim.pattern.test(line) && !allowedTruthQualifiers.test(line)) {
        failures.push(`${file}:${index + 1}: ${claim.label}: ${line.trim()}`);
      }
    }
  });
}

const compatibilitySource = existsSync("crates/desktoplab-compatibility/src/seed_catalog.rs")
  ? readFileSync("crates/desktoplab-compatibility/src/seed_catalog.rs", "utf8")
  : "";
if (/runtime\.future[\s\S]{0,240}is_downloadable_now/i.test(compatibilitySource)) {
  failures.push("crates/desktoplab-compatibility/src/seed_catalog.rs: runtime.future must not be locally downloadable");
}

const publicRuntimeClaims = readRequiredFile("docs-public/runtime-and-provider-support.md");
const requiredFrontierClaimTerms = [
  "`architected`",
  "`detected`",
  "`configured`",
  "`certified`",
  "`blocked`",
  "`unsupported`",
  "model quality",
  "runtime performance",
  "repository-context retrieval",
  "installed-app agent behavior",
];
for (const term of requiredFrontierClaimTerms) {
  if (!publicRuntimeClaims.includes(term)) {
    failures.push(
      `docs-public/runtime-and-provider-support.md: missing frontier-local claim boundary term ${term}`,
    );
  }
}

for (const term of [
  "private-installed-evidence-gated",
  "iterative model/tool/observation loop",
  "installed-app live evidence",
]) {
  if (!publicRuntimeClaims.includes(term)) {
    failures.push(`docs-public/runtime-and-provider-support.md: missing agent-truth boundary term ${term}`);
  }
}

for (const [index, line] of publicRuntimeClaims.split("\n").entries()) {
  if (!/\bfrontier-local capable\b/i.test(line)) continue;
  const hasCertificationScope =
    /\bcertified\b/i.test(line) && /\bhost\b/i.test(line) && /\bruntime\b/i.test(line) && /\bmodel\b/i.test(line);
  if (!hasCertificationScope) {
    failures.push(
      `docs-public/runtime-and-provider-support.md:${index + 1}: frontier-local capable requires certified host/runtime/model scope`,
    );
  }
}

if (failures.length > 0) {
  console.error("Product claim guard failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log("Product claim guard passed");

function collectGuardedFiles(entry) {
  if (!existsSync(entry)) return [];
  if (/\.(md|ts|tsx|mjs)$/.test(entry)) return [entry];
  return readdirSync(entry, { withFileTypes: true }).flatMap((dirent) => {
    const child = path.join(entry, dirent.name);
    if (dirent.isDirectory()) return collectGuardedFiles(child);
    return /\.(md|ts|tsx|mjs)$/.test(child) ? [child] : [];
  });
}

function readRequiredFile(file) {
  if (!existsSync(file)) {
    failures.push(`${file}: required claim-boundary document is missing`);
    return "";
  }
  return readFileSync(file, "utf8");
}

function isIgnoredSource(file) {
  return (
    /\.test\.(ts|tsx)$/.test(file) ||
    /\.spec\.(ts|tsx)$/.test(file) ||
    file.includes("/api/") ||
    file.includes("/domain/")
  );
}

function isIgnoredLine(line) {
  return (
    /data-ui-state=/.test(line) ||
    /refetchInterval:/.test(line) ||
    /state\s*===\s*["']ready["']/.test(line) ||
    /state:\s*["']ready["']/.test(line)
  );
}
