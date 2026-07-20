export type DesktopPlatform = "macos" | "windows" | "linux" | "unknown";

export function detectDesktopPlatform(
  platform = typeof navigator === "undefined" ? "" : navigator.platform,
  userAgent = typeof navigator === "undefined" ? "" : navigator.userAgent,
): DesktopPlatform {
  const identity = `${platform} ${userAgent}`.toLowerCase();
  if (identity.includes("mac")) return "macos";
  if (identity.includes("win")) return "windows";
  if (identity.includes("linux") || identity.includes("x11")) return "linux";
  return "unknown";
}
