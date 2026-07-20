export function assertNoConfiguredEntitlements(configured) {
  if (!configured || Array.isArray(configured) || typeof configured !== "object") {
    throw new Error("macOS entitlements must be a plist dictionary");
  }
  const keys = Object.keys(configured);
  if (keys.length > 0) {
    throw new Error(`macOS signing does not yet support configured entitlements: ${keys.join(", ")}`);
  }
}
