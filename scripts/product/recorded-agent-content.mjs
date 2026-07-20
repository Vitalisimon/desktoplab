export function requestedTextMatches(actual, expected) {
  if (actual === expected) return true;
  if (typeof actual !== "string" || typeof expected !== "string") return false;
  return withoutTerminalNewline(actual) === withoutTerminalNewline(expected)
    && terminalNewlineCount(actual) <= 1
    && terminalNewlineCount(expected) <= 1;
}

function withoutTerminalNewline(value) {
  return value.replace(/\r?\n$/, "");
}

function terminalNewlineCount(value) {
  return /\r?\n$/.test(value) ? 1 : 0;
}
