// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { render, screen } from "@testing-library/react";
import type { UpdateStatusSnapshot } from "../../api/types";
import { UpdateStatusPanel } from "./UpdateStatusPanel";

test("shows update channel current version and verified state without manifest internals", () => {
  render(<UpdateStatusPanel updateStatus={status()} />);

  expect(screen.getByRole("heading", { name: "Updates" })).toBeInTheDocument();
  expect(screen.getByText("Stable")).toBeInTheDocument();
  expect(screen.getByText("DesktopLab 0.1.0")).toBeInTheDocument();
  expect(screen.getByText("No update available.")).toBeInTheDocument();
  expect(screen.queryByText("https://releases.desktoplab.ai")).not.toBeInTheDocument();
  expect(screen.queryByText("manifest", { exact: false })).not.toBeInTheDocument();
  expect(screen.queryByText("token", { exact: false })).not.toBeInTheDocument();
});

test("shows failed update checks as human explanation without install controls", () => {
  render(
    <UpdateStatusPanel
      updateStatus={{
        ...status(),
        state: "failed",
        message: "DesktopLab could not verify the update signature. The current app is still usable.",
        canInstall: false,
      }}
    />,
  );

  expect(screen.getByText("Needs attention")).toBeInTheDocument();
  expect(screen.getByText("DesktopLab could not verify the update signature. The current app is still usable.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /install/i })).not.toBeInTheDocument();
});

test("update status panel stays small and presentation-only", () => {
  const source = readFileSync("src/features/settings/UpdateStatusPanel.tsx", "utf8");

  expect(source.split("\n").length).toBeLessThan(120);
});

function status(): UpdateStatusSnapshot {
  return {
    channel: "stable",
    currentVersion: "0.1.0",
    state: "up_to_date",
    message: "No update available.",
    canInstall: false,
  };
}
