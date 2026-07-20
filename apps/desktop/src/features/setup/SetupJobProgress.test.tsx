// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { SetupJobProgress } from "./SetupJobProgress";
import type { SetupJobProgressSnapshot } from "../../api/types";

test("renders setup jobs with distinct execution states", () => {
  render(<SetupJobProgress progress={progress()} />);

  expect(screen.getByText("Install local runner")).toBeInTheDocument();
  expect(screen.getByText("Running")).toBeInTheDocument();
  expect(screen.getByText("Download coding model")).toBeInTheDocument();
  expect(screen.getByText("Blocked")).toBeInTheDocument();
  expect(screen.getByText("Check local runner")).toBeInTheDocument();
  expect(screen.getByText("Completed")).toBeInTheDocument();
  expect(screen.queryByText("runtime.install:runtime.ollama")).not.toBeInTheDocument();
});

test("failed setup job exposes the next recovery action", () => {
  render(<SetupJobProgress progress={failedProgress()} />);

  expect(screen.getByText("Failed")).toBeInTheDocument();
  expect(screen.getByText("Free 12 GB of disk space, then retry.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /retry setup/i })).toBeEnabled();
});

test("renders runtime execution phase copy when backend provides it", () => {
  render(<SetupJobProgress progress={phaseProgress()} />);

  expect(screen.getByText("Download installer")).toBeInTheDocument();
  expect(screen.queryByText("download_failed_retryable")).not.toBeInTheDocument();
});

test("running model download exposes cancel action", () => {
  const onCancel = vi.fn();
  render(<SetupJobProgress progress={cancelProgress()} onCancel={onCancel} />);

  fireEvent.click(screen.getByRole("button", { name: /cancel download/i }));

  expect(onCancel).toHaveBeenCalledWith("job.model");
});

test("model setup failures use distinct friendly recovery copy", () => {
  render(<SetupJobProgress progress={modelFailureProgress()} />);

  expect(screen.getByText("Reconnect to the internet, then try setup again.")).toBeInTheDocument();
  expect(screen.getByText("Free up disk space, then try setup again.")).toBeInTheDocument();
  expect(screen.getByText("Verify the local runner before downloading this model.")).toBeInTheDocument();
  expect(screen.getByText("This model reference is not safe to run.")).toBeInTheDocument();
  expect(screen.getByText("Choose a compatible local runner for this model.")).toBeInTheDocument();
  expect(screen.getByText("Start this model download again from the beginning.")).toBeInTheDocument();
  expect(screen.queryByText("offline")).not.toBeInTheDocument();
  expect(screen.queryByText("non_retryable")).not.toBeInTheDocument();
  expect(screen.queryByText("user_action")).not.toBeInTheDocument();
});

function progress(): SetupJobProgressSnapshot {
  return {
    sequence: 42,
    jobs: [
      {
        id: "runtime.install:runtime.ollama",
        label: "Install local runner",
        status: "running",
        progressPercent: 36,
      },
      {
        id: "model.download:model.qwen-coder",
        label: "Download coding model",
        status: "blocked",
        progressPercent: 0,
        nextAction: "Waiting for local registry unlock.",
      },
      {
        id: "runtime.verify:runtime.ollama",
        label: "Check local runner",
        status: "completed",
        progressPercent: 100,
      },
    ],
  };
}

function phaseProgress(): SetupJobProgressSnapshot {
  return {
    sequence: 44,
    jobs: [
      {
        id: "job.1",
        label: "Runtime install",
        phaseLabel: "Download installer",
        status: "failed",
        progressPercent: 25,
        nextAction: "Check the connection and retry.",
        retryAvailable: true,
      },
    ],
  };
}

function failedProgress(): SetupJobProgressSnapshot {
  return {
    sequence: 43,
    jobs: [
      {
        id: "model.download:model.qwen-coder",
        label: "Download coding model",
        status: "failed",
        progressPercent: 18,
        nextAction: "Free 12 GB of disk space, then retry.",
        retryAvailable: true,
      },
    ],
  };
}

function cancelProgress(): SetupJobProgressSnapshot {
  return {
    sequence: 45,
    jobs: [
      {
        id: "job.model",
        label: "Download coding model",
        status: "running",
        progressPercent: 32,
        cancelAvailable: true,
      },
    ],
  };
}

function modelFailureProgress(): SetupJobProgressSnapshot {
  return {
    sequence: 46,
    jobs: [
      failure("job.offline", "offline"),
      failure("job.disk", "insufficient disk"),
      failure("job.runtime", "runtime_not_verified"),
      failure("job.unsafe", "unsafe model reference"),
      failure("job.unsupported", "unsupported runtime"),
      failure("job.resume", "resume unsupported"),
    ],
  };
}

function failure(id: string, nextAction: string): SetupJobProgressSnapshot["jobs"][number] {
  return {
    id,
    label: "Download coding model",
    status: "blocked",
    progressPercent: 0,
    nextAction,
  };
}
