// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import type { WorkspaceFilePreviewResponse } from "../../api/types";
import { FilePreviewPanel } from "./FilePreviewPanel";

test("renders safe text preview with truncation and redaction state", () => {
  render(
    <FilePreviewPanel
      path="src/main.rs"
      preview={{
        workspaceId: "workspace.desktoplab",
        path: "src/main.rs",
        state: "text",
        text: "fn main() {}\nAPI_KEY=[REDACTED_SECRET]",
        deniedReason: null,
        originalBytes: 2000,
        originalLines: 120,
        returnedLines: 2,
        truncated: true,
      }}
    />,
  );

  expect(screen.getByText("src/main.rs")).toBeInTheDocument();
  expect(screen.getByText("fn main() {}")).toBeInTheDocument();
  expect(screen.getByText("API_KEY=[REDACTED_SECRET]")).toBeInTheDocument();
  expect(screen.getByText("Preview limited")).toBeInTheDocument();
  expect(screen.getByText("Text preview")).toBeInTheDocument();
  expect(screen.getByText("Redacted")).toBeInTheDocument();
  expect(screen.getByText("2 of 120 lines")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Open file" })).not.toBeInTheDocument();
});

test("shows open action only when backend preview provides it", () => {
  render(
    <FilePreviewPanel
      path="src/main.rs"
      preview={{
        workspaceId: "workspace.desktoplab",
        path: "src/main.rs",
        state: "text",
        text: "fn main() {}",
        deniedReason: null,
        originalBytes: 12,
        originalLines: 1,
        returnedLines: 1,
        truncated: false,
        openAction: { label: "Open file" },
      }}
    />,
  );

  expect(screen.getByRole("button", { name: "Open file" })).toBeInTheDocument();
});

test.each([
  [
    "binary",
    {
      workspaceId: "workspace.desktoplab",
      path: "image.bin",
      state: "binary",
      text: null,
      deniedReason: null,
      originalBytes: 1024,
      originalLines: 0,
      returnedLines: 0,
      truncated: false,
    } satisfies WorkspaceFilePreviewResponse,
    "Binary file, 1024 bytes.",
  ],
  [
    "denied",
    {
      workspaceId: "workspace.desktoplab",
      path: ".env",
      state: "denied",
      text: null,
      deniedReason: "local_only_path",
      originalBytes: 0,
      originalLines: 0,
      returnedLines: 0,
      truncated: false,
    } satisfies WorkspaceFilePreviewResponse,
    "Protected local file.",
  ],
])("renders %s preview state", (_state, preview, expectedText) => {
  render(<FilePreviewPanel path={preview.path} preview={preview} />);

  expect(screen.getByText(expectedText)).toBeInTheDocument();
});

test("can close preview independently from the drawer", async () => {
  const onClose = vi.fn();
  render(
    <FilePreviewPanel
      path="src/main.rs"
      onClose={onClose}
      preview={{
        workspaceId: "workspace.desktoplab",
        path: "src/main.rs",
        state: "text",
        text: "fn main() {}",
        deniedReason: null,
        originalBytes: 12,
        originalLines: 1,
        returnedLines: 1,
        truncated: false,
      }}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Close preview" }));

  expect(onClose).toHaveBeenCalledTimes(1);
});
