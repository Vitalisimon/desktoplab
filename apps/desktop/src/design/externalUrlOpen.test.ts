// @vitest-environment jsdom

import { describe, expect, test, vi } from "vitest";

describe("openExternalUrl", () => {
  test("uses the native external-url command inside the packaged shell", async () => {
    vi.resetModules();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { openExternalUrl } = await import("./externalUrlOpen");

    await openExternalUrl("https://auth.openai.com/codex/device");

    expect(invoke).toHaveBeenCalledWith("open_external_url", {
      url: "https://auth.openai.com/codex/device",
    });
  });

  test("falls back to browser window open in web tests and dev", async () => {
    vi.resetModules();
    vi.unstubAllGlobals();
    const open = vi.spyOn(window, "open").mockImplementation(() => null);
    const { openExternalUrl } = await import("./externalUrlOpen");

    await openExternalUrl("https://auth.openai.com/codex/device");

    expect(open).toHaveBeenCalledWith(
      "https://auth.openai.com/codex/device",
      "_blank",
      "noopener,noreferrer",
    );
    open.mockRestore();
  });
});
