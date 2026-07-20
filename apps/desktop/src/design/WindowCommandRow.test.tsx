// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { WindowCommandRow } from "./WindowCommandRow";
import { detectDesktopPlatform } from "./desktopPlatform";

test("detects supported desktop platforms from browser identity", () => {
  expect(detectDesktopPlatform("MacIntel", "Mozilla/5.0")).toBe("macos");
  expect(detectDesktopPlatform("Win32", "Mozilla/5.0")).toBe("windows");
  expect(detectDesktopPlatform("Linux x86_64", "Mozilla/5.0 X11")).toBe("linux");
  expect(detectDesktopPlatform("", "DesktopLabWebView")).toBe("unknown");
});

test("reserves traffic-light space only for macOS overlay chrome", () => {
  const { rerender } = renderCommandRow("macos");

  expect(screen.getByTestId("window-command-row")).toHaveClass("grid-cols-[104px_minmax(0,1fr)_auto]");
  expect(screen.getByTestId("window-command-row")).toHaveAttribute("data-tauri-drag-region");
  expect(screen.getByTestId("window-chrome-left-cluster")).toHaveClass("pl-[92px]");
  expect(screen.getByText("Desktop client")).toBeInTheDocument();

  rerender(commandRow("windows"));

  expect(screen.getByTestId("window-command-row")).toHaveClass("grid-cols-[40px_minmax(0,1fr)_auto]");
  expect(screen.getByTestId("window-command-row")).not.toHaveAttribute("data-tauri-drag-region");
  expect(screen.getByTestId("window-chrome-left-cluster")).toHaveClass("pl-3");
  expect(screen.queryByText("Desktop client")).not.toBeInTheDocument();
  expect(screen.getByText("Open a repository")).toBeInTheDocument();

  rerender(commandRow("linux"));
  expect(screen.getByTestId("window-command-row")).toHaveClass("grid-cols-[40px_minmax(0,1fr)_auto]");
  expect(screen.getByTestId("window-command-row")).toHaveAttribute("data-platform", "linux");
});

function renderCommandRow(platform: "macos" | "windows" | "linux") {
  return render(commandRow(platform));
}

function commandRow(platform: "macos" | "windows" | "linux") {
  return (
    <WindowCommandRow
      leftOpen
      rightOpen={false}
      terminalOpen={false}
      changesOpen={false}
      hasWorkspace={false}
      workspace={null}
      onToggleLeft={vi.fn()}
      onToggleRight={vi.fn()}
      onToggleTerminal={vi.fn()}
      onToggleChanges={vi.fn()}
      openTargets={[]}
      onOpenTarget={vi.fn()}
      platform={platform}
    />
  );
}
