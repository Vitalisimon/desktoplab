// @vitest-environment jsdom

import { describe, expect, test, vi } from "vitest";

describe("openRepositoryInFileManager", () => {
  test("opens the active repository path through the native shell command", async () => {
    vi.resetModules();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { openRepositoryInFileManager } = await import("./repositoryOpen");

    await openRepositoryInFileManager("/Users/name/project");

    expect(invoke).toHaveBeenCalledWith("open_repository_in_file_manager", { path: "/Users/name/project" });
  });

  test("does not call the native command outside the packaged shell", async () => {
    vi.resetModules();
    vi.unstubAllGlobals();
    const invoke = vi.fn().mockResolvedValue(undefined);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const { openRepositoryInFileManager } = await import("./repositoryOpen");

    await openRepositoryInFileManager("/Users/name/project");

    expect(invoke).not.toHaveBeenCalled();
  });
});

test("reads native repository targets and opens only the selected target id", async () => {
  vi.resetModules();
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  const invoke = vi.fn().mockResolvedValueOnce([{ id: "file_manager", label: "Finder", kind: "file_manager" }]).mockResolvedValueOnce(undefined);
  vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
  const { repositoryOpenTargets, openRepositoryInTarget } = await import("./repositoryOpen");

  expect(await repositoryOpenTargets()).toEqual([{ id: "file_manager", label: "Finder", kind: "file_manager" }]);
  await openRepositoryInTarget("/Users/name/project", "file_manager");

  expect(invoke).toHaveBeenNthCalledWith(1, "repository_open_targets");
  expect(invoke).toHaveBeenNthCalledWith(2, "open_repository_in_target", { path: "/Users/name/project", targetId: "file_manager" });
});
