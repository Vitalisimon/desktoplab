const MIN_AUTOMATIC_TOKEN_LENGTH = 4;

export function buildDynamicForbiddenPatterns({ home, hostname, blocklist }) {
  const patterns = [];
  addBoundedPattern(patterns, home, MIN_AUTOMATIC_TOKEN_LENGTH);
  addBoundedPattern(patterns, hostname, MIN_AUTOMATIC_TOKEN_LENGTH);

  for (const value of blocklist?.split(",") ?? []) {
    addBoundedPattern(patterns, value, 1);
  }

  return patterns;
}

export function decodeTextCandidate(bytes) {
  if (bytes.includes(0)) {
    return null;
  }

  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return null;
  }

  const disallowedControls = [...text].filter((character) => {
    const code = character.codePointAt(0);
    return code < 32 && character !== "\n" && character !== "\r" && character !== "\t";
  }).length;
  const controlLimit = Math.max(2, Math.floor(text.length * 0.01));
  return disallowedControls > controlLimit ? null : text;
}

function addBoundedPattern(patterns, rawValue, minimumLength) {
  const value = rawValue?.trim();
  if (!value || value.length < minimumLength) {
    return;
  }

  patterns.push(
    new RegExp(
      `(?:^|[^\\p{L}\\p{N}_])${escapeRegExp(value)}(?=$|[^\\p{L}\\p{N}_])`,
      "iu",
    ),
  );
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
