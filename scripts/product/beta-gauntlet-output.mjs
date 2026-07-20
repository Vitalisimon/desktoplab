export function hasUnexpectedSkip(output) {
  const tapSkipCounts = [...output.matchAll(/^\s*(?:ℹ|#)?\s*skipped\s+(\d+)\s*$/gim)]
    .map((match) => Number.parseInt(match[1], 10));

  if (tapSkipCounts.length > 0) {
    return tapSkipCounts.some((count) => count > 0);
  }

  const allowed = /narrow skipped by design/i;
  return /\b(?:skipped|skip)\b/i.test(output) && !allowed.test(output);
}
