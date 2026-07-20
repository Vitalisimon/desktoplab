import { isPrivateReleasePath } from "../release/public-release-source-core.mjs";

const CANONICAL_PUBLIC_REPOSITORY = "vitalisimon/desktoplab";

export function directSourceAuditRequired({ args = [], repository = "" } = {}) {
  return args.includes("--direct-source")
    || repository.trim().toLowerCase() === CANONICAL_PUBLIC_REPOSITORY;
}

export function isForbiddenPublicTrackedPath(path) {
  return isPrivateReleasePath(path)
    || path.startsWith("dist/")
    || path.startsWith("target/")
    || path.startsWith("node_modules/")
    || path.includes("test-artifacts/")
    || path.includes("test-results/")
    || /\.(dmg|msi|AppImage|deb|rpm)$/.test(path);
}
