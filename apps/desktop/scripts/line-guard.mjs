import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const guardedFiles = [
  ["src/design/AppFrame.tsx", 180],
  ["src/design/AppDrawer.tsx", 180],
  ["src/design/WindowCommandRow.tsx", 90],
  ["src/design/RepositoryInspector.tsx", 100],
  ["src/app/App.tsx", 80],
  ["src/app/AppRoutes.tsx", 120],
  ["src/api/auth.ts", 40],
  ["src/api/client.ts", 320],
  ["src/api/events.ts", 140],
  ["src/api/terminalEvents.ts", 120],
  ["src/api/types.ts", 40],
  ["src/api/setupTypes.ts", 140],
  ["src/api/runtimeTypes.ts", 120],
  ["src/api/modelTypes.ts", 100],
  ["src/api/workspaceTypes.ts", 220],
  ["src/api/sessionTypes.ts", 160],
  ["src/api/approvalTypes.ts", 120],
  ["src/design/OperationalPrimitives.tsx", 150],
  ["src/design/icons.ts", 90],
  ["src/features/setup/SetupWizard.tsx", 260],
  ["src/features/setup/RecommendationDetails.tsx", 190],
  ["src/features/setup/RecommendationView.tsx", 180],
  ["src/features/setup/SetupJobProgress.tsx", 160],
  ["src/features/setup/SetupReadinessResult.tsx", 120],
  ["src/features/productization/ProvidersFeature.tsx", 150],
  ["src/features/productization/RuntimeModelFeature.tsx", 150],
  ["src/features/productization/AgentWorkspaceFeature.tsx", 220],
  ["src/features/productization/AgentConversation.tsx", 120],
  ["src/features/productization/GitOperationsFeature.tsx", 220],
  ["src/features/productization/WorkspaceContextFeature.tsx", 220],
  ["src/features/productization/ExtensionsFeature.tsx", 180],
  ["src/features/productization/DiagnosticsFeature.tsx", 180],
  ["src/features/settings/ProductizationSettingsSummary.tsx", 140],
  ["tests/product/auditHelpers.ts", 120],
];

let failed = false;

for (const [file, limit] of guardedFiles) {
  const source = readFileSync(new URL(`../${file}`, import.meta.url), "utf8");
  const logicalLines = source
    .split("\n")
    .filter((line) => line.trim().length > 0 && !line.trim().startsWith("//")).length;

  if (logicalLines > limit) {
    failed = true;
    console.error(`${file} has ${logicalLines} logical lines; limit is ${limit}`);
  }
}

function walkSourceFiles(dir) {
  return readdirSync(dir)
    .flatMap((entry) => {
      const path = join(dir, entry);
      const stat = statSync(path);
      if (stat.isDirectory()) return walkSourceFiles(path);
      return /\.(ts|tsx)$/.test(entry) ? [path] : [];
    });
}

const appRoot = fileURLToPath(new URL("..", import.meta.url));

for (const file of walkSourceFiles(join(appRoot, "src"))) {
  if (file.endsWith("src/design/icons.ts")) continue;
  const source = readFileSync(file, "utf8");
  if (source.includes("from \"lucide-react\"") || source.includes("from 'lucide-react'")) {
    failed = true;
    console.error(`${relative(appRoot, file)} imports lucide-react directly; import from src/design/icons instead`);
  }
}

if (failed) {
  process.exit(1);
}
