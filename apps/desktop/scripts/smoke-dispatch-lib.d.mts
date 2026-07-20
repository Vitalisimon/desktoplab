export interface ResolvePlaywrightCliPathOptions {
  appDir: string;
  workspaceRoot: string;
  exists?: (candidate: string) => boolean;
}

export function resolvePlaywrightCliPath(
  options: ResolvePlaywrightCliPathOptions,
): string;

export interface CleanupGeneratedArtifactsAfterPassOptions {
  appDir: string;
  status: number;
  env?: Record<string, string | undefined>;
  rm?: (target: string, options: { force: boolean; recursive: boolean }) => void;
}

export type CleanupGeneratedArtifactsAfterPassStatus =
  | "cleaned"
  | "preserved_failed_run"
  | "preserved_by_env";

export function cleanupGeneratedArtifactsAfterPass(
  options: CleanupGeneratedArtifactsAfterPassOptions,
): CleanupGeneratedArtifactsAfterPassStatus;
