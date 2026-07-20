// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import type { DesktopLabApiClient } from "../../api/client";
import type { WorkspaceFilePreviewResponse, WorkspaceFileTreeResponse } from "../../api/types";
import { AppProviders } from "../../app/AppProviders";
import { RepositoryFileTree } from "./RepositoryFileTree";

test("loads repository files and opens safe file previews", async () => {
  const previewWorkspaceFile = vi.fn<(workspaceId: string, path: string) => Promise<WorkspaceFilePreviewResponse>>().mockResolvedValue({
    workspaceId: "workspace.desktoplab",
    path: "src/main.rs",
    state: "text",
    text: "fn main() {}\nAPI_KEY=[REDACTED_SECRET]",
    deniedReason: null,
    originalBytes: 42,
    originalLines: 2,
    returnedLines: 2,
    truncated: false,
  });
  render(
    <AppProviders apiClient={clientFor(previewWorkspaceFile)}>
      <RepositoryFileTree workspaceId="workspace.desktoplab" />
    </AppProviders>,
  );

  fireEvent.click(await screen.findByRole("button", { name: "src" }));
  expect(screen.getByRole("button", { name: "main.rs" })).toBeInTheDocument();
  expect(screen.getByText("Protected")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "main.rs" }));

  expect(await screen.findByText("fn main() {}")).toBeInTheDocument();
  expect(screen.getByText("API_KEY=[REDACTED_SECRET]")).toBeInTheDocument();
  expect(previewWorkspaceFile).toHaveBeenCalledWith("workspace.desktoplab", "src/main.rs");
});

test("renders repository files as collapsed folders by default", async () => {
  const previewWorkspaceFile = vi.fn<(workspaceId: string, path: string) => Promise<WorkspaceFilePreviewResponse>>().mockResolvedValue({
    workspaceId: "workspace.desktoplab",
    path: "src/main.rs",
    state: "text",
    text: "fn main() {}",
    deniedReason: null,
    originalBytes: 12,
    originalLines: 1,
    returnedLines: 1,
    truncated: false,
  });
  render(
    <AppProviders apiClient={clientFor(previewWorkspaceFile)}>
      <RepositoryFileTree workspaceId="workspace.desktoplab" />
    </AppProviders>,
  );

  const srcFolder = await screen.findByRole("button", { name: "src" });
  expect(screen.queryByRole("button", { name: "main.rs" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Button.tsx" })).not.toBeInTheDocument();

  fireEvent.click(srcFolder);
  expect(screen.getByRole("button", { name: "main.rs" })).toBeVisible();
  expect(screen.getByRole("button", { name: "components" })).toBeVisible();
  expect(screen.queryByRole("button", { name: "Button.tsx" })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "components" }));
  expect(screen.getByRole("button", { name: "Button.tsx" })).toBeVisible();

  fireEvent.click(screen.getByRole("button", { name: "main.rs" }));

  expect(await screen.findByText("fn main() {}")).toBeInTheDocument();
  expect(previewWorkspaceFile).toHaveBeenCalledWith("workspace.desktoplab", "src/main.rs");
});

test("opens selected file preview as the primary right drawer surface", async () => {
  const previewWorkspaceFile = vi.fn<(workspaceId: string, path: string) => Promise<WorkspaceFilePreviewResponse>>().mockResolvedValue({
    workspaceId: "workspace.desktoplab",
    path: "src/main.rs",
    state: "text",
    text: "fn main() {}",
    deniedReason: null,
    originalBytes: 12,
    originalLines: 1,
    returnedLines: 1,
    truncated: false,
  });
  render(
    <AppProviders apiClient={clientFor(previewWorkspaceFile)}>
      <RepositoryFileTree workspaceId="workspace.desktoplab" />
    </AppProviders>,
  );

  fireEvent.click(await screen.findByRole("button", { name: "src" }));
  const file = screen.getByRole("button", { name: "main.rs" });
  fireEvent.doubleClick(file);

  expect(await screen.findByText("fn main() {}")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Show repository tree" })).toBeInTheDocument();
  expect(screen.getByTestId("repository-tree-subdrawer")).toHaveAttribute("aria-hidden", "true");

  fireEvent.click(screen.getByRole("button", { name: "Show repository tree" }));

  expect(screen.getByTestId("repository-tree-subdrawer")).toHaveAttribute("aria-hidden", "false");
  expect(screen.getByRole("button", { name: "main.rs" })).toBeVisible();
  expect(previewWorkspaceFile).toHaveBeenCalledWith("workspace.desktoplab", "src/main.rs");
});

test("previews files from keyboard activation and keeps non-text states metadata only", async () => {
  const previewWorkspaceFile = vi.fn<(workspaceId: string, path: string) => Promise<WorkspaceFilePreviewResponse>>().mockImplementation(async (_workspaceId, path) => {
    if (path === "image.bin") {
      return {
        workspaceId: "workspace.desktoplab",
        path,
        state: "binary",
        text: null,
        deniedReason: null,
        originalBytes: 1024,
        originalLines: 0,
        returnedLines: 0,
        truncated: false,
      };
    }
    return {
      workspaceId: "workspace.desktoplab",
      path,
      state: "text",
      text: "line 1",
      deniedReason: null,
      originalBytes: 2048,
      originalLines: 250,
      returnedLines: 1,
      truncated: true,
    };
  });
  render(
    <AppProviders apiClient={clientFor(previewWorkspaceFile)}>
      <RepositoryFileTree workspaceId="workspace.desktoplab" />
    </AppProviders>,
  );

  const largeFile = await screen.findByRole("button", { name: "large.log" });
  largeFile.focus();
  fireEvent.keyDown(largeFile, { key: "Enter" });

  expect(await screen.findByText("Preview limited")).toBeInTheDocument();
  expect(screen.getByText("1 of 250 lines")).toBeInTheDocument();
  expect(previewWorkspaceFile).toHaveBeenCalledWith("workspace.desktoplab", "large.log");

  fireEvent.click(screen.getByRole("button", { name: "Show repository tree" }));
  const binaryFile = screen.getByRole("button", { name: "image.bin" });
  binaryFile.focus();
  fireEvent.keyDown(binaryFile, { key: " " });

  expect(await screen.findByText("Binary file, 1024 bytes.")).toBeInTheDocument();
  expect(screen.getByText("Binary metadata")).toBeInTheDocument();
  expect(previewWorkspaceFile).toHaveBeenCalledWith("workspace.desktoplab", "image.bin");
  expect(screen.getByText("Protected")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: ".env" })).not.toBeInTheDocument();
});

function clientFor(
  previewWorkspaceFile: (workspaceId: string, path: string) => Promise<WorkspaceFilePreviewResponse>,
): DesktopLabApiClient {
  return {
    listWorkspaceFiles: vi.fn<() => Promise<WorkspaceFileTreeResponse>>().mockResolvedValue({
      workspaceId: "workspace.desktoplab",
      degraded: false,
      degradedReasons: [],
      limits: { maxEntries: 200, maxDepth: 8 },
      entries: [
        { path: "src", kind: "directory", protection: "readable" },
        { path: "src/main.rs", kind: "file", protection: "readable" },
        { path: "src/components", kind: "directory", protection: "readable" },
        { path: "src/components/Button.tsx", kind: "file", protection: "readable" },
        { path: "large.log", kind: "file", protection: "readable" },
        { path: "image.bin", kind: "file", protection: "readable" },
        { path: ".env", kind: "hidden_file", protection: "protected" },
      ],
    }),
    previewWorkspaceFile,
  } as unknown as DesktopLabApiClient;
}
