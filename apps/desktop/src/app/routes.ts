export type InitialRouteInput = {
  readiness: "starting" | "ready" | "degraded" | "blocked";
  setupState?: "not_started" | "in_progress" | "ready" | "blocked";
  hasWorkspace: boolean;
  activeApprovalCount: number;
  activeSessionCount: number;
};

export type AppRoute =
  | "setup"
  | "workspaces"
  | "changes"
  | "jobs"
  | "sessions"
  | "approvals"
  | "providers"
  | "models"
  | "agent"
  | "context"
  | "extensions"
  | "diagnostics"
  | "settings";

export function selectInitialRoute(input: InitialRouteInput): AppRoute {
  if (input.setupState && input.setupState !== "ready") return "setup";
  if (input.readiness === "blocked" || input.readiness === "degraded") return "setup";
  if (!input.hasWorkspace) return "workspaces";
  return "agent";
}

export function guardRouteForSetup(route: AppRoute, input: InitialRouteInput): AppRoute {
  if (!requiresReadySetup(route)) return route;
  return input.setupState === "ready" || input.readiness === "ready" ? route : "setup";
}

function requiresReadySetup(route: AppRoute): boolean {
  return ["workspaces", "changes", "sessions", "approvals", "agent", "context"].includes(route);
}
