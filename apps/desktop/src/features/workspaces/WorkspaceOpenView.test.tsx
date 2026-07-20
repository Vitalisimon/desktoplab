// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { WorkspaceOpenRequest, WorkspaceSnapshot } from "../../api/types";
import { repositoryPathPlaceholder, WorkspaceOpenView } from "./WorkspaceOpenView";

vi.mock("./repositoryFolderPicker", () => ({
  chooseRepositoryFolder: vi.fn(),
}));

test("uses platform-appropriate repository path examples", () => {
  expect(repositoryPathPlaceholder("Win32")).toBe(String.raw`C:\Users\name\project`);
  expect(repositoryPathPlaceholder("MacIntel")).toBe("/Users/name/project");
  expect(repositoryPathPlaceholder("Linux x86_64")).toBe("/home/name/project");
});

import { chooseRepositoryFolder } from "./repositoryFolderPicker";

test("opens an existing repository path and returns the backend snapshot", async () => {
  const onOpened = vi.fn();
  const openWorkspace = vi.fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>().mockResolvedValue(snapshot());

  render(
    <AppProviders apiClient={{ openWorkspace } as unknown as DesktopLabApiClient}>
      <WorkspaceOpenView onOpened={onOpened} />
    </AppProviders>,
  );

  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/repo/desktoplab" } });
  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));

  await waitFor(() => expect(onOpened).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "workspace.desktoplab" })));
  expect(openWorkspace).toHaveBeenCalledWith({ path: "/repo/desktoplab" });
});

test("uses plain project-folder copy and explains invalid folders", async () => {
  const openWorkspace = vi.fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>().mockRejectedValue(
    new Error("not a git repository"),
  );

  render(
    <AppProviders apiClient={{ openWorkspace } as unknown as DesktopLabApiClient}>
      <WorkspaceOpenView onOpened={vi.fn()} />
    </AppProviders>,
  );

  expect(screen.getByRole("heading", { name: "Open a project folder" })).toBeInTheDocument();
  expect(screen.getByText("Choose an existing folder. If it is not a Git repository yet, DesktopLab can initialize it after you confirm.")).toBeInTheDocument();

  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/repo/not-git" } });
  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));

  expect(await screen.findByText("This folder is not a Git repository yet. DesktopLab can initialize Git here before opening it.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Initialize Git and open" })).toBeInTheDocument();
});

test("initializes git only after explicit confirmation for a normal folder", async () => {
  const onOpened = vi.fn();
  const openWorkspace = vi
    .fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>()
    .mockRejectedValueOnce(new Error("not a git repository"))
    .mockResolvedValue(snapshot());

  render(
    <AppProviders apiClient={{ openWorkspace } as unknown as DesktopLabApiClient}>
      <WorkspaceOpenView onOpened={onOpened} />
    </AppProviders>,
  );

  fireEvent.change(screen.getByLabelText("Repository path"), { target: { value: "/repo/empty-project" } });
  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));
  fireEvent.click(await screen.findByRole("button", { name: "Initialize Git and open" }));

  await waitFor(() => expect(onOpened).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "workspace.desktoplab" })));
  expect(openWorkspace).toHaveBeenNthCalledWith(1, { path: "/repo/empty-project" });
  expect(openWorkspace).toHaveBeenNthCalledWith(2, { path: "/repo/empty-project", initializeGit: true });
});

test("opens the folder picker when no path is provided", async () => {
  vi.mocked(chooseRepositoryFolder).mockResolvedValue("/repo/desktoplab");
  const onOpened = vi.fn();
  const openWorkspace = vi.fn<(request: WorkspaceOpenRequest) => Promise<WorkspaceSnapshot>>().mockResolvedValue(snapshot());

  render(
    <AppProviders apiClient={{ openWorkspace } as unknown as DesktopLabApiClient}>
      <WorkspaceOpenView onOpened={onOpened} />
    </AppProviders>,
  );

  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));

  await waitFor(() => expect(chooseRepositoryFolder).toHaveBeenCalled());
  await waitFor(() => expect(onOpened).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "workspace.desktoplab" })));
  expect(openWorkspace).toHaveBeenCalledWith({ path: "/repo/desktoplab" });
});

function snapshot(): WorkspaceSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    displayName: "desktoplab",
    rootPath: "/repo/desktoplab",
    gitDirPath: "/repo/desktoplab/.git",
    apiState: "clean",
    statusEntries: [],
    diffText: "",
    checkpointStatus: "ready",
    canCheckpointRiskyExecution: true,
  };
}
