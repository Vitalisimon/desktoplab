import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const PERMITTED_LICENSES = new Set([
  "0BSD",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "BSL-1.0",
  "CC-BY-4.0",
  "CDLA-Permissive-2.0",
  "ISC",
  "MIT",
  "MIT-0",
  "Unicode-3.0",
  "Unlicense",
  "Zlib",
]);
const KNOWN_RESTRICTED_LICENSES = new Set([
  "AGPL-3.0-only",
  "AGPL-3.0-or-later",
  "GPL-2.0-only",
  "GPL-2.0-or-later",
  "GPL-3.0-only",
  "GPL-3.0-or-later",
  "LGPL-2.1-only",
  "LGPL-2.1-or-later",
  "LGPL-3.0-only",
  "LGPL-3.0-or-later",
  "SSPL-1.0",
]);
const PERMITTED_EXCEPTIONS = new Set(["LLVM-exception"]);
const LEGACY_EXPRESSIONS = new Map([
  ["MIT/Apache-2.0", "MIT OR Apache-2.0"],
  ["Unlicense/MIT", "Unlicense OR MIT"],
]);

export const sensitivePatterns = [
  ["private-key", /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/],
  ["aws-access-key", /\bAKIA[0-9A-Z]{16}\b/],
  ["openai-secret", /\bsk-[A-Za-z0-9_-]{20,}\b/],
  ["github-token", /\bgh[ps]_[A-Za-z0-9]{30,}\b/],
];

export function sha256File(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

export function assessLicenseExpression(expression) {
  if (!expression || typeof expression !== "string") {
    return { expression: expression ?? null, accepted: false, identifiers: [], unknown: ["MISSING"], restricted: [] };
  }
  const normalized = LEGACY_EXPRESSIONS.get(expression.trim()) ?? expression.trim();
  const parser = new SpdxParser(tokenize(normalized));
  let tree;
  try {
    tree = parser.parse();
  } catch (error) {
    return { expression, normalized, accepted: false, identifiers: [], unknown: [error.message], restricted: [] };
  }
  const identifiers = [...collectIdentifiers(tree)].sort();
  const unknown = identifiers.filter((id) => !PERMITTED_LICENSES.has(id) && !KNOWN_RESTRICTED_LICENSES.has(id) && !PERMITTED_EXCEPTIONS.has(id));
  const restricted = identifiers.filter((id) => KNOWN_RESTRICTED_LICENSES.has(id));
  return {
    expression,
    normalized,
    accepted: unknown.length === 0 && evaluate(tree),
    identifiers,
    unknown,
    restricted,
  };
}

export function classifyLicenses(packages) {
  const entries = packages.map((pkg) => ({ ...pkg, policy: assessLicenseExpression(pkg.license) }));
  const findings = entries
    .filter((entry) => !entry.policy.accepted)
    .map((entry) => ({ ecosystem: entry.ecosystem, name: entry.name, version: entry.version, license: entry.license, policy: entry.policy }));
  return { status: findings.length === 0 ? "pass" : "fail", packageCount: entries.length, findings, entries };
}

export function classifyAuditAdvisories({ cargoAudits, npmAudit }) {
  const findings = [];
  const notices = [];
  for (const { scope, report } of cargoAudits) {
    for (const item of report.vulnerabilities?.list ?? []) {
      findings.push({
        ecosystem: "cargo",
        scope,
        id: item.advisory?.id ?? "unknown",
        severity: item.advisory?.informational ?? "vulnerability",
        status: "unclassified",
      });
    }
    for (const [kind, items] of Object.entries(report.warnings ?? {})) {
      for (const item of items) {
        const entry = {
          ecosystem: "cargo",
          scope,
          id: item.advisory?.id ?? kind,
          severity: kind,
          status: kind === "unmaintained" ? "tracked-notice" : "unclassified",
        };
        (kind === "unmaintained" ? notices : findings).push(entry);
      }
    }
  }
  for (const [name, item] of Object.entries(npmAudit.vulnerabilities ?? {})) {
    findings.push({
      ecosystem: "npm",
      scope: "workspace",
      id: name,
      severity: item.severity,
      status: "unclassified",
    });
  }
  return { status: findings.length === 0 ? "pass" : "fail", findings, notices };
}

export function scanText({ label, text, privateValues = [], allowedKinds = [] }) {
  const allowed = new Set(allowedKinds);
  const findings = [];
  for (const [kind, pattern] of sensitivePatterns) {
    if (!allowed.has(kind) && pattern.test(text)) findings.push({ label, kind });
  }
  for (const value of privateValues.filter(Boolean)) {
    if (text.includes(value)) findings.push({ label, kind: "private-path" });
  }
  return findings;
}

export function buildCycloneDx({ commit, version, cargoMetadata, npmPackages, lockHashes }) {
  const cargoRefs = new Map(cargoMetadata.packages.map((pkg) => [pkg.id, `pkg:cargo/${pkg.name}@${pkg.version}`]));
  const cargoComponents = cargoMetadata.packages.map((pkg) => component("cargo", pkg.name, pkg.version, pkg.license));
  const npmComponents = npmPackages.map((pkg) => component("npm", pkg.name, pkg.version, pkg.license));
  const dependencies = (cargoMetadata.resolve?.nodes ?? []).map((node) => ({
    ref: cargoRefs.get(node.id) ?? node.id,
    dependsOn: node.dependencies.map((id) => cargoRefs.get(id) ?? id).sort(),
  }));
  return {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    version: 1,
    metadata: {
      timestamp: new Date().toISOString(),
      component: { type: "application", name: "DesktopLab", version, "bom-ref": `desktoplab:${commit}` },
      properties: [
        { name: "desktoplab:sourceCommit", value: commit },
        ...lockHashes.map((lock) => ({ name: `desktoplab:lock:${lock.path}`, value: lock.sha256 })),
      ],
    },
    components: [...cargoComponents, ...npmComponents].sort((left, right) => left["bom-ref"].localeCompare(right["bom-ref"])),
    dependencies,
  };
}

function component(ecosystem, name, version, license) {
  const encodedName = ecosystem === "npm" ? name.replace(/^@/, "%40") : name;
  return {
    type: "library",
    name,
    version,
    purl: `pkg:${ecosystem}/${encodedName}@${version}`,
    "bom-ref": `pkg:${ecosystem}/${encodedName}@${version}`,
    licenses: [{ expression: license }],
  };
}

function tokenize(expression) {
  const tokens = expression.match(/\(|\)|\bAND\b|\bOR\b|\bWITH\b|[A-Za-z0-9.+-]+/g) ?? [];
  const compact = expression.replace(/\s+/g, "");
  if (tokens.join("").replace(/\s+/g, "") !== compact) throw new Error("INVALID_SPDX_SYNTAX");
  return tokens;
}

class SpdxParser {
  constructor(tokens) {
    this.tokens = tokens;
    this.index = 0;
  }

  parse() {
    const tree = this.parseOr();
    if (this.index !== this.tokens.length) throw new Error("UNEXPECTED_SPDX_TOKEN");
    return tree;
  }

  parseOr() {
    let left = this.parseAnd();
    while (this.take("OR")) left = { op: "OR", left, right: this.parseAnd() };
    return left;
  }

  parseAnd() {
    let left = this.parseWith();
    while (this.take("AND")) left = { op: "AND", left, right: this.parseWith() };
    return left;
  }

  parseWith() {
    const base = this.parsePrimary();
    if (!this.take("WITH")) return base;
    const exception = this.tokens[this.index++];
    if (!exception || ["AND", "OR", "WITH", "(", ")"].includes(exception)) throw new Error("INVALID_SPDX_EXCEPTION");
    return { op: "WITH", base, exception };
  }

  parsePrimary() {
    if (this.take("(")) {
      const nested = this.parseOr();
      if (!this.take(")")) throw new Error("UNCLOSED_SPDX_GROUP");
      return nested;
    }
    const id = this.tokens[this.index++];
    if (!id || ["AND", "OR", "WITH", ")"].includes(id)) throw new Error("MISSING_SPDX_IDENTIFIER");
    return { op: "LICENSE", id };
  }

  take(token) {
    if (this.tokens[this.index] !== token) return false;
    this.index += 1;
    return true;
  }
}

function collectIdentifiers(node, output = new Set()) {
  if (node.op === "LICENSE") output.add(node.id);
  else if (node.op === "WITH") {
    collectIdentifiers(node.base, output);
    output.add(node.exception);
  } else {
    collectIdentifiers(node.left, output);
    collectIdentifiers(node.right, output);
  }
  return output;
}

function evaluate(node) {
  if (node.op === "LICENSE") return PERMITTED_LICENSES.has(node.id);
  if (node.op === "WITH") return evaluate(node.base) && PERMITTED_EXCEPTIONS.has(node.exception);
  if (node.op === "AND") return evaluate(node.left) && evaluate(node.right);
  return evaluate(node.left) || evaluate(node.right);
}
