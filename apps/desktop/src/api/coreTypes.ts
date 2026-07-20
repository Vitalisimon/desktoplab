export type HealthResponse = {
  status: "healthy" | "draining";
};

export type ReadinessResponse = {
  state: "starting" | "ready" | "degraded" | "blocked";
  degradedReasons?: string[];
};

export type VersionResponse = {
  productVersion: string;
  apiVersion: "v1";
};

export type OperationalStatus = "ready" | "degraded" | "blocked" | "running" | "not_installed";

export type TrustLevel = "local" | "verified" | "unverified";

export type RepairAction = {
  id: string;
  label: string;
  description: string;
};

export type ApiErrorCode = "unauthorized" | "not_found" | "network_error" | "backend_error";

export class DesktopLabApiError extends Error {
  readonly code: ApiErrorCode;
  readonly status: number;

  constructor(code: ApiErrorCode, message: string, status = 0) {
    super(message);
    this.name = "DesktopLabApiError";
    this.code = code;
    this.status = status;
  }
}
