import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("../", import.meta.url).pathname);
const srcRoot = path.join(repoRoot, "src");

const scanRoots = ["app", "design", "features"].map((folder) => path.join(srcRoot, folder));
const workspaceRoot = path.resolve(repoRoot, "../..");
const forbiddenItalianCopy = /\b(Richiedi|Approva|Scritture|Accesso completo|Apri|Chiudi|Salva|Annulla|Caricamento|Impostazioni|Diagnostica)\b/;
const slowInteractionDuration = /duration-(300|500|700|1000)/;
const rawBackendCodes = [
  "gpu_probe_unavailable",
  "vram_probe_unavailable",
  "driver_probe_deferred_to_v2",
  "runtime_not_ready",
  "runtime_not_verified",
  "model_not_verified",
  "model_not_reported_by_runtime",
  "backend_readiness_not_verified",
  "runtime_and_model_not_verified",
  "local_inference_not_configured",
  "local_inference_failed",
];

const allowedRawCodeFiles = [
  "features/setup/hardwareWarningCopy.ts",
  "features/setup/setupFailureCopy.ts",
  "features/productization/AgentConversation.tsx",
  "features/productization/conversationDisplay.ts",
  "features/productization/DiagnosticsFeature.tsx",
  "features/settings/LocalDiagnosticsPanel.tsx",
  "features/settings/DiagnosticsBundlePanel.tsx",
];

const forbiddenRawStateDisplays = [
  /Runtime install \{\s*install\.state\s*\}/,
  /Model download \{\s*download\.state\s*\}/,
  /is \$\{\s*download\.state\s*\}/,
  /\{[^}]*\.state\[0\][^}]*\.slice\(1\)[^}]*\}/,
  /\{[^}]*\.compatibility\}/,
  /\{[^}]*backend\.state\}/,
];

const requiredProductCopy = [
  { file: "domain/displayNames.ts", text: "Not connected" },
  { file: "domain/displayNames.ts", text: "Needs approval" },
  { file: "domain/displayNames.ts", text: "Disabled by policy" },
  { file: "features/setup/SetupJobProgress.tsx", text: "Blocked" },
  { file: "features/setup/SetupJobProgress.tsx", text: "Failed" },
  { file: "features/productization/RouteModelMenu.tsx", text: 'option.backendKind === "local" && option.status === "available"' },
  { file: "features/productization/AgentComposer.tsx", text: 'return route.modelDisplayName ?? "No model selected"' },
];

const failures = [];
const styles = readFileSync(path.join(srcRoot, "styles.css"), "utf8");

for (const file of walk(scanRoots)) {
  const relativePath = path.relative(srcRoot, file);
  if (!isScannedSourceFile(relativePath)) continue;
  const source = readFileSync(file, "utf8");

  if (!isAllowedRawCodeFile(relativePath)) {
    for (const code of rawBackendCodes) {
      if (source.includes(code)) {
        failures.push(`${relativePath}: raw backend code '${code}' must stay behind display-copy helpers or diagnostics.`);
      }
    }
  }

  if (!isAllowedRawStateDisplayFile(relativePath)) {
    for (const pattern of forbiddenRawStateDisplays) {
      if (pattern.test(source)) {
        failures.push(`${relativePath}: render product state through approved display labels instead of raw state values.`);
      }
    }
  }

  if (forbiddenItalianCopy.test(source)) {
    failures.push(`${relativePath}: product copy must be English.`);
  }
  if (slowInteractionDuration.test(source)) {
    failures.push(`${relativePath}: interactive transitions must stay at or below 200ms.`);
  }
}

const domainPolicy = readFileSync(path.join(workspaceRoot, "crates/desktoplab-domain/src/policy.rs"), "utf8");
if (forbiddenItalianCopy.test(domainPolicy)) {
  failures.push("crates/desktoplab-domain/src/policy.rs: approval labels must be English.");
}

for (const requirement of requiredProductCopy) {
  const source = readFileSync(path.join(srcRoot, requirement.file), "utf8");
  if (!source.includes(requirement.text)) {
    failures.push(`${requirement.file}: missing approved product copy '${requirement.text}'.`);
  }
}

for (const requirement of [
  "scrollbar-color: transparent transparent",
  "background-color: transparent",
  "*:hover::-webkit-scrollbar-thumb",
  "*:focus-within::-webkit-scrollbar-thumb",
]) {
  if (!styles.includes(requirement)) {
    failures.push(`styles.css: missing quiet scrollbar contract '${requirement}'.`);
  }
}

if (failures.length > 0) {
  console.error("Visual vocabulary guard failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log("Visual vocabulary guard passed.");

function walk(roots) {
  const files = [];
  for (const root of roots) collect(root, files);
  return files;
}

function collect(entry, files) {
  const stat = statSync(entry);
  if (stat.isDirectory()) {
    for (const child of readdirSync(entry)) collect(path.join(entry, child), files);
    return;
  }
  files.push(entry);
}

function isScannedSourceFile(relativePath) {
  return /\.(ts|tsx)$/.test(relativePath) && !relativePath.endsWith(".test.ts") && !relativePath.endsWith(".test.tsx");
}

function isAllowedRawCodeFile(relativePath) {
  return allowedRawCodeFiles.includes(relativePath);
}

function isAllowedRawStateDisplayFile(relativePath) {
  return relativePath.includes("Diagnostics") || relativePath.includes("diagnostics");
}
