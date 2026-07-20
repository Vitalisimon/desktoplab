import { existsSync } from "node:fs";

export function resolveEvidencePath({ explicitPath, candidates = [], exists = existsSync }) {
  if (explicitPath) return explicitPath;
  return candidates.find((candidate) => exists(candidate)) ?? null;
}
