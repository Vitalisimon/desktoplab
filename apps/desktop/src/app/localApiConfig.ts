import { DesktopLabApiClient } from "../api/client";
import { FetchTransport } from "../api/transport";

export type LocalApiBootstrap = {
  baseUrl: string;
  authToken: string;
  source: "tauri" | "browser-dev-fallback";
};

export function localApiBaseUrl() {
  return localEnv("VITE_DESKTOPLAB_API_BASE_URL") ?? "http://127.0.0.1:1421";
}

export function localApiFallbackAuthToken() {
  return localEnv("VITE_DESKTOPLAB_API_AUTH_TOKEN") ?? "";
}

export async function resolveLocalApiBootstrap(): Promise<LocalApiBootstrap> {
  if (isTauriRuntime()) {
    const { invoke } = await import("@tauri-apps/api/core");
    const bootstrap = await invoke<{ baseUrl: string; authToken: string }>("local_api_bootstrap");
    assertPackagedBootstrap(bootstrap);
    return {
      baseUrl: bootstrap.baseUrl,
      authToken: bootstrap.authToken,
      source: "tauri",
    };
  }

  assertBrowserDevFallbackAllowed();
  return {
    baseUrl: localApiBaseUrl(),
    authToken: localApiFallbackAuthToken(),
    source: "browser-dev-fallback",
  };
}

export async function createDesktopLabApiClient() {
  const bootstrap = await resolveLocalApiBootstrap();
  return new DesktopLabApiClient({
    authToken: bootstrap.authToken,
    transport: new FetchTransport(bootstrap.baseUrl),
  });
}

function isTauriRuntime() {
  if (typeof window === "undefined") return false;
  return "__TAURI_INTERNALS__" in window;
}

function assertPackagedBootstrap(bootstrap: { baseUrl?: string; authToken?: string }) {
  if (!bootstrap.baseUrl?.startsWith("http://127.0.0.1:")) {
    throw new Error("Packaged DesktopLab requires a native loopback local API.");
  }
  if (!bootstrap.authToken?.trim()) {
    throw new Error("Packaged DesktopLab requires a native local API token.");
  }
}

function assertBrowserDevFallbackAllowed() {
  if (localEnv("VITE_DESKTOPLAB_ALLOW_BROWSER_DEV_FALLBACK") === "1") {
    return;
  }
  throw new Error("Browser dev local API fallback requires explicit opt-in.");
}

function localEnv(name: string) {
  const env = (import.meta as ImportMeta & { env?: Record<string, string | undefined> }).env;
  return env?.[name] ?? testProcessEnv(name);
}

function testProcessEnv(name: string) {
  const processLike = globalThis as typeof globalThis & {
    process?: { env?: Record<string, string | undefined> };
  };
  return processLike.process?.env?.[name];
}
