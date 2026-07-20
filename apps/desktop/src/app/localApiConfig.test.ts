// @vitest-environment jsdom

import { beforeEach, expect, test, vi } from "vitest";
import {
  createDesktopLabApiClient,
  localApiBaseUrl,
  resolveLocalApiBootstrap,
} from "./localApiConfig";

beforeEach(() => {
  window.localStorage.clear();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

test("browser fallback is blocked unless explicitly allowed", async () => {
  expect(localApiBaseUrl()).toBe("http://127.0.0.1:1421");
  await expect(resolveLocalApiBootstrap()).rejects.toThrow(
    "Browser dev local API fallback requires explicit opt-in.",
  );
});

test("browser fallback keeps the existing unsafe no-auth smoke base url when explicitly allowed", async () => {
  vi.stubEnv("VITE_DESKTOPLAB_ALLOW_BROWSER_DEV_FALLBACK", "1");

  await expect(resolveLocalApiBootstrap()).resolves.toEqual({
    baseUrl: "http://127.0.0.1:1421",
    authToken: "",
    source: "browser-dev-fallback",
  });
});

test("browser fallback can use explicit packaged smoke token without localStorage", async () => {
  vi.stubEnv("VITE_DESKTOPLAB_ALLOW_BROWSER_DEV_FALLBACK", "1");
  vi.stubEnv("VITE_DESKTOPLAB_API_AUTH_TOKEN", "packaged-smoke-token");

  await expect(resolveLocalApiBootstrap()).resolves.toEqual({
    baseUrl: "http://127.0.0.1:1421",
    authToken: "packaged-smoke-token",
    source: "browser-dev-fallback",
  });
  expect(window.localStorage.getItem("desktoplab.localApiToken")).toBeNull();
});

test("packaged shell bootstrap uses native command instead of localStorage", async () => {
  window.localStorage.setItem("desktoplab.localApiToken", "stale-token");
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  vi.doMock("@tauri-apps/api/core", () => ({
    invoke: vi.fn().mockResolvedValue({
      baseUrl: "http://127.0.0.1:49152",
      authToken: "native-token",
    }),
  }));

  const bootstrap = await resolveLocalApiBootstrap();

  expect(bootstrap).toEqual({
    baseUrl: "http://127.0.0.1:49152",
    authToken: "native-token",
    source: "tauri",
  });
  expect(window.localStorage.getItem("desktoplab.localApiToken")).toBe("stale-token");
});

test("packaged shell bootstrap fails closed without a native token", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  vi.doMock("@tauri-apps/api/core", () => ({
    invoke: vi.fn().mockResolvedValue({
      baseUrl: "http://127.0.0.1:49152",
      authToken: "",
    }),
  }));

  await expect(resolveLocalApiBootstrap()).rejects.toThrow(
    "Packaged DesktopLab requires a native local API token.",
  );
});

test("client factory keeps native token in memory only", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  vi.doMock("@tauri-apps/api/core", () => ({
    invoke: vi.fn().mockResolvedValue({
      baseUrl: "http://127.0.0.1:49152",
      authToken: "native-token",
    }),
  }));

  const client = await createDesktopLabApiClient();

  expect(client).toBeDefined();
  expect(window.localStorage.getItem("desktoplab.localApiToken")).toBeNull();
});
