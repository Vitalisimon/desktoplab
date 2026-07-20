import { DesktopLabApiError } from "./types";
import { authHeaders, backendErrorMessage } from "./auth";
import type { ApiTransport, TransportResponse } from "./transport";

export class ApiRequester {
  constructor(
    private readonly transport: ApiTransport,
    private readonly authToken: string,
  ) {}

  get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  async request<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<T> {
    let response: TransportResponse;
    try {
      response = await this.transport.request({
        method,
        path,
        headers: authHeaders(this.authToken),
        body,
      });
    } catch (error) {
      throw new DesktopLabApiError(
        "network_error",
        error instanceof Error ? error.message : "Local API request failed",
      );
    }

    if (response.status === 401) {
      throw new DesktopLabApiError("unauthorized", "Local API token was rejected", 401);
    }
    if (response.status === 404) {
      throw new DesktopLabApiError("not_found", "Local API route was not found", 404);
    }
    if (response.status >= 400) {
      throw new DesktopLabApiError("backend_error", backendErrorMessage(response.body), response.status);
    }

    return response.body as T;
  }
}
