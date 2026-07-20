export function signatureState(details, notarized = false) {
  if (notarized) return "notarized";
  if (/Authority=Developer ID Application:/m.test(details) && /TeamIdentifier=\S+/m.test(details) && /flags=.*runtime/m.test(details)) {
    return "developer_id";
  }
  if (/Signature=adhoc/m.test(details)) return "adhoc_dev";
  return "invalid";
}

export function assertSignatureMode(state, mode) {
  const accepted = mode === "dev"
    ? ["adhoc_dev", "developer_id", "notarized"]
    : mode === "release"
      ? ["developer_id", "notarized"]
      : ["notarized"];
  if (!accepted.includes(state)) throw new Error(`${mode} bundle verification rejects ${state} signature state`);
}
