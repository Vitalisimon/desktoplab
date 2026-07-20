// @vitest-environment jsdom
import { describe, expect, test, vi } from "vitest";

describe("startWindowDrag", () => {
  test("starts native window drag only inside the packaged shell", async () => {
    vi.resetModules();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { startWindowDrag } = await import("./windowDrag");

    startWindowDrag(mouseEventFor(document.createElement("div")));
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("start_window_drag"));
  });

  test("does not start window drag from toolbar buttons", async () => {
    vi.resetModules();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { startWindowDrag } = await import("./windowDrag");

    startWindowDrag(mouseEventFor(document.createElement("button")));

    expect(invoke).not.toHaveBeenCalled();
  });

  test("toggles native maximize from non-interactive toolbar center", async () => {
    vi.resetModules();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { toggleWindowMaximized } = await import("./windowDrag");

    toggleWindowMaximized(mouseEventFor(document.createElement("div")));
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("toggle_window_maximized"));
  });
});

function mouseEventFor(target: Element) {
  return {
    button: 0,
    preventDefault: vi.fn(),
    target,
  } as never;
}
