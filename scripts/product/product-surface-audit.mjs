import { existsSync, readFileSync } from "node:fs";
import {
  validatePlatformClaims,
  validateSecurityReportingClaims,
  validateSourcePublicationClaims,
} from "./product-surface-audit-core.mjs";
import { currentRepositoryVisibilityMode } from "./repository-visibility-mode.mjs";

const MODES = new Set(["internal", "public-export"]);
const args = parseArgs(process.argv.slice(2));

const inventoryPath = "docs/product-surface-truth-inventory.md";
const hasPrivateInventory = existsSync(inventoryPath);

if (!hasPrivateInventory && args.mode === "internal") {
  console.error("Product surface audit failed: private inventory is not present in an internal checkout");
  process.exit(1);
}

const failures = [];

if (hasPrivateInventory) {
  auditPrivateInventory(readFileSync(inventoryPath, "utf8"));
}

if (args.mode === "public-export") {
  auditPublicDocs();
}

if (failures.length > 0) {
  console.error("Product surface audit failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`Product surface audit passed (${args.mode})`);

function auditPrivateInventory(source) {
  if (!/^Status: active-for-24\.9$/m.test(source)) {
    failures.push("product surface inventory must be active-for-24.9 before beta truth closure");
  }

  for (const line of source.split(/\r?\n/)) {
    if (!line.startsWith("|") || line.includes("---") || line.includes("Surface")) continue;
    const cells = line
      .split("|")
      .slice(1, -1)
      .map((cell) => cell.trim().toLowerCase());
    const [surface, audience, state] = cells;
    if ((audience === "base-user" || audience === "public") && state === "prepared-only") {
      failures.push(`${surface}: ${audience} surface is prepared-only`);
    }
  }
}

function auditPublicDocs() {
  const publicFiles = [
    "docs-public/README.md",
    "docs-public/install.md",
    "docs-public/platform-support.md",
    "docs-public/runtime-and-provider-support.md",
    "docs-public/security.md",
    "docs-public/troubleshooting.md",
    "docs-public/linux-code-signing-policy.md",
    "docs-public/windows-code-signing-policy.md"
  ];
  for (const file of publicFiles) {
    if (!existsSync(file)) failures.push(`missing public document: ${file}`);
  }

  const runtimeDoc = readPublicText("docs-public/runtime-and-provider-support.md");
  requireText(
    runtimeDoc,
    "No provider is advertised as ready when only adapter contracts exist.",
    "provider claims must stay evidence-bound"
  );
  requireText(
    runtimeDoc,
    "OpenAI subscription account / Codex local bridge",
    "public docs must describe subscription bridge separately"
  );
  requireText(
    runtimeDoc,
    "Not publicly claimed as supported",
    "runtime docs must not advertise uncertified runtimes"
  );

  const platformDoc = readPublicText("docs-public/platform-support.md");
  const releaseClaims = readPublicJson("docs-public/release-claims.json");
  failures.push(...validatePlatformClaims(platformDoc, releaseClaims));
  for (const file of ["README.md", "docs-public/public-export-gate.md", "docs-public/security.md", "docs-public/supply-chain.md"]) {
    failures.push(...validateSourcePublicationClaims(readPublicText(file), releaseClaims).map((failure) => `${file}: ${failure}`));
  }

  const windowsSigningDoc = readPublicText("docs-public/windows-code-signing-policy.md");
  requirePattern(
    windowsSigningDoc,
    /This lane proves that DesktopLab can build, sign, install, launch and uninstall\s+on the tested host\. It does not establish public publisher identity/,
    "self-signed Windows evidence must stay non-public"
  );
  requirePattern(
    windowsSigningDoc,
    /(?:Acceptance by\s+SignPath Foundation has not been requested or granted\.|Application was submitted to SignPath Foundation on \d{4}-\d{2}-\d{2}\.\s+Acceptance has\s+not been granted\.)/,
    "pre-acceptance SignPath state must remain explicit"
  );

  const linuxSigningDoc = readPublicText("docs-public/linux-code-signing-policy.md");
  requirePattern(
    linuxSigningDoc,
    /This policy is prepared, not accepted\./,
    "prepared Linux signing must not be presented as accepted"
  );
  requirePattern(
    linuxSigningDoc,
    /The AppImage currently uses the detached Sigstore bundle\./,
    "Linux signing policy must disclose the AppImage signature format"
  );
  requirePattern(
    linuxSigningDoc,
    /if DesktopLab later operates an APT\s+repository, the repository must additionally publish authenticated\s+`InRelease` or `Release` plus `Release\.gpg` metadata\./,
    "standalone deb signing must not be presented as APT repository trust"
  );

  const securityDoc = readPublicText("docs-public/security.md");
  failures.push(...validateSecurityReportingClaims(securityDoc));
}

function readPublicText(file) {
  return existsSync(file) ? readFileSync(file, "utf8") : "";
}

function readPublicJson(file) {
  if (!existsSync(file)) {
    failures.push(`missing public document: ${file}`);
    return {};
  }
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch {
    failures.push(`invalid JSON public document: ${file}`);
    return {};
  }
}

function requireText(source, needle, message) {
  if (!source.includes(needle)) failures.push(message);
}

function requirePattern(source, pattern, message) {
  if (!pattern.test(source)) failures.push(message);
}

function parseArgs(rawArgs) {
  const parsed = { mode: currentRepositoryVisibilityMode() };
  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = rawArgs[index];
    if (arg === "--mode") parsed.mode = rawArgs[++index] ?? "";
    else if (arg.startsWith("--mode=")) parsed.mode = arg.slice("--mode=".length);
    else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(2);
    }
  }
  if (!MODES.has(parsed.mode)) {
    console.error(`Invalid mode: ${parsed.mode}`);
    process.exit(2);
  }
  return parsed;
}
