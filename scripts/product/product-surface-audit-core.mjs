const PLATFORM_ROWS = {
  macosAppleSilicon: "macOS Apple Silicon",
  linuxX64: "Linux x64",
  windowsX64: "Windows x64",
};

export function classifyAvailability(value) {
  const normalized = value.trim().toLowerCase().replace(/\s+/g, " ");
  const isNotPublic =
    /\bnot (?:currently )?public\b/.test(normalized) ||
    /\bnot publicly available\b/.test(normalized) ||
    /\bnot (?:currently )?published\b/.test(normalized) ||
    /\bpublication (?:is )?pending\b/.test(normalized) ||
    /\bnot live\b/.test(normalized) ||
    /\bunavailable\b/.test(normalized);

  if (isNotPublic && /\bcandidate\b/.test(normalized)) return "candidate_not_public";
  if (isNotPublic) return "not_public";
  return "public";
}

export function validateSourcePublicationClaims(source, releaseClaims) {
  const failures = [];
  const expected = releaseClaims?.sourceAvailability;
  if (!expected) return ["machine-readable source publication claim is missing"];
  const actual = classifyAvailability(sourceStatus(source));
  if (actual !== expected) failures.push(`source publication is ${actual}, expected ${expected}`);
  if (expected !== "public" && [
    /\bpublic source repository is live\b/i,
    /\bDesktopLab source is public\b/i,
    /\bthe historyless repository is public\b/i,
    /Status:\s*(?:public source published|public-source-active|historyless public repository live)/i,
  ].some((pattern) => pattern.test(source))) {
    failures.push("source publication copy claims a live repository while publication is pending");
  }
  return failures;
}

function sourceStatus(source) {
  return source
    .split(/\r?\n/)
    .find((line) => /^Status:\s*/i.test(line.trim()))
    ?? source;
}

export function validatePlatformClaims(source, releaseClaims) {
  const failures = [];
  const rows = parsePlatformTable(source);

  for (const [claimKey, platformName] of Object.entries(PLATFORM_ROWS)) {
    const expected = releaseClaims?.platforms?.[claimKey]?.publicAvailability;
    const row = rows.get(platformName.toLowerCase());

    if (!expected) {
      failures.push(`${platformName}: missing machine-readable publicAvailability claim`);
      continue;
    }
    if (!row) {
      failures.push(`${platformName}: missing public platform table row`);
      continue;
    }

    const actual = classifyAvailability(row.publicAvailability);
    if (actual !== expected) {
      failures.push(
        `${platformName}: public availability is ${actual}, expected ${expected} from release-claims.json`,
      );
    }
  }

  const hasMacPublicationBoundary = source
    .split(/\r?\n/)
    .some(
      (line) =>
        /macos/i.test(line) &&
        /sign(?:ed|ing)/i.test(line) &&
        /notari[sz](?:ed|ation)/i.test(line) &&
        /does not\b.*\bauthori[sz]e publication/i.test(line),
    );
  if (!hasMacPublicationBoundary) {
    failures.push("macOS publication boundary must separate signing and notarization from publication");
  }

  return failures;
}

export function validateSecurityReportingClaims(source) {
  const normalized = source.replace(/\s+/g, " ");
  const failures = [];

  const namesPrivateReporting = /Private Vulnerability Reporting\b/i.test(normalized);
  const reportsEnabled = /\bchannel is enabled\b/i.test(normalized) ||
    /Private Vulnerability Reporting is enabled\b/i.test(normalized);
  const publicationPending = /Private Vulnerability Reporting is not currently available\b[^.]*\bpublic repository publication is pending\b/i.test(normalized) &&
    /\bchannel must be enabled and reverified\b[^.]*\bnew repository is published\b/i.test(normalized);
  if (!namesPrivateReporting || (!reportsEnabled && !publicationPending)) {
    failures.push("Private Vulnerability Reporting must be enabled or explicitly pending activation");
  }

  const proofPending = /no external private test report\b[^.]*\bverified\b/i.test(normalized);
  const historicalProof = publicationPending &&
    /historical external non-collaborator report\b/i.test(normalized) &&
    /\bprivate end-to-end path\b/i.test(normalized) &&
    /\breceived\b/i.test(normalized) && /\btriaged\b/i.test(normalized) && /\bclosed\b/i.test(normalized) &&
    /without public disclosure\b/i.test(normalized);
  const proofVerified = reportsEnabled && !historicalProof &&
    /external non-collaborator report\b/i.test(normalized) &&
    /\b(?:end-to-end|reporter-to-maintainer)\b[^.]*\b(?:verified|completed)\b|\b(?:verified|completed)\b[^.]*\b(?:end-to-end|reporter-to-maintainer)\b/i.test(normalized) &&
    /\breceived\b/i.test(normalized) &&
    /\btriaged\b/i.test(normalized) &&
    /\bclosed\b/i.test(normalized) &&
    /without public disclosure\b/i.test(normalized);
  if ([proofPending, proofVerified, historicalProof].filter(Boolean).length !== 1) {
    failures.push("the external private test report must be explicitly pending or verified end to end");
  }

  const keepsBinaryBoundary =
    /public beta binaries remain blocked\b/i.test(normalized) ||
    /does not claim released-binary support\b/i.test(normalized) ||
    /no public\b[^.]*\bbinar(?:y|ies)\b[^.]*\b(?:released|shared|available)\b/i.test(normalized);
  if (!keepsBinaryBoundary) {
    failures.push("binary release boundary missing: public beta binaries remain blocked or released-binary support must be explicitly unclaimed");
  }

  return failures;
}

function parsePlatformTable(source) {
  const rows = new Map();
  const lines = source.split(/\r?\n/);
  const headerIndex = lines.findIndex((line) => {
    const cells = parseTableRow(line).map((cell) => cell.toLowerCase());
    return cells.includes("platform") && cells.includes("public availability");
  });

  if (headerIndex < 0) return rows;

  for (const line of lines.slice(headerIndex + 1)) {
    if (!line.trim().startsWith("|")) break;
    const cells = parseTableRow(line);
    if (cells.length < 2 || cells.every((cell) => /^:?-+:?$/.test(cell))) continue;
    const [platform, publicAvailability] = cells;
    if (platform) rows.set(platform.toLowerCase(), { publicAvailability });
  }

  return rows;
}

function parseTableRow(line) {
  if (!line.trim().startsWith("|")) return [];
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}
