// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { SetupReadinessResult } from "./SetupReadinessResult";
import type { ReadinessResponse } from "../../api/types";

test("ready setup exposes the open repository action", () => {
  const onOpenRepository = vi.fn();

  render(<SetupReadinessResult readiness={{ state: "ready" }} onOpenRepository={onOpenRepository} />);

  fireEvent.click(screen.getByRole("button", { name: /open repository/i }));
  expect(onOpenRepository).toHaveBeenCalledTimes(1);
  expect(screen.getByText("Ready")).toBeInTheDocument();
});

test("degraded setup is not presented as complete", () => {
  render(
    <SetupReadinessResult readiness={readiness("degraded", ["LM Studio runtime is unavailable."])} onOpenRepository={vi.fn()} />,
  );

  expect(screen.getByText("Degraded")).toBeInTheDocument();
  expect(screen.queryByText("Ready")).not.toBeInTheDocument();
  expect(screen.getByText("LM Studio runtime is unavailable.")).toBeInTheDocument();
});

test("blocked setup explains the next step and disables repository opening", () => {
  render(
    <SetupReadinessResult readiness={readiness("blocked", ["No compatible runtime is available."])} onOpenRepository={vi.fn()} />,
  );

  expect(screen.getByText("Blocked")).toBeInTheDocument();
  expect(screen.getByText("No compatible runtime is available.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /open repository/i })).toBeDisabled();
});

test("blocked setup hides backend readiness codes behind user copy", () => {
  render(
    <SetupReadinessResult
      readiness={readiness("blocked", ["backend_readiness_not_verified", "model_not_reported_by_runtime"])}
      onOpenRepository={vi.fn()}
    />,
  );

  expect(screen.getByText("Verify the local runner and model before opening a repository.")).toBeInTheDocument();
  expect(screen.getByText("The local runner does not report this model yet.")).toBeInTheDocument();
  expect(screen.queryByText("backend_readiness_not_verified")).not.toBeInTheDocument();
  expect(screen.queryByText("model_not_reported_by_runtime")).not.toBeInTheDocument();
});

test("blocked setup hides combined readiness code behind user copy", () => {
  render(
    <SetupReadinessResult readiness={readiness("blocked", ["runtime_and_model_not_verified"])} onOpenRepository={vi.fn()} />,
  );

  expect(screen.getByText("Verify the local runner and model before opening a repository.")).toBeInTheDocument();
  expect(screen.queryByText("runtime_and_model_not_verified")).not.toBeInTheDocument();
});

test("offline setup block points to reconnect or cached installer path", () => {
  render(
    <SetupReadinessResult readiness={readiness("blocked", ["network unavailable"])} onOpenRepository={vi.fn()} />,
  );

  expect(screen.getByText("Reconnect to the internet, then try setup again.")).toBeInTheDocument();
  expect(screen.queryByText("network unavailable")).not.toBeInTheDocument();
});

function readiness(state: ReadinessResponse["state"], degradedReasons: string[]): ReadinessResponse {
  return { state, degradedReasons };
}
