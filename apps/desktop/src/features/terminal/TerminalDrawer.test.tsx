// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import type { BackendEventFrame } from "../../api/events";
import { TerminalDrawer } from "./TerminalDrawer";
import type { TerminalCommandResponse } from "../../api/types";

test("keeps an empty terminal compact and expands once output exists", () => {
  const response: TerminalCommandResponse = {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "printf ok",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [
      {
        eventId: "terminal.local.output",
        kind: "output",
        stdout: "ok",
        stderr: "",
        status: "exited",
        exitCode: 0,
        stdoutTruncated: false,
        redacted: false,
      },
    ],
  };
  const { rerender } = render(<TerminalDrawer open height={132} workspacePath="/Users/name/project" sessionLabel="Agent session" onClose={() => undefined} />);

  expect(screen.getByRole("complementary", { name: "Terminal" })).toHaveStyle({ height: "132px" });
  expect(screen.getByText("project")).toBeInTheDocument();
  expect(screen.getByText("project %")).toBeInTheDocument();
  expect(screen.queryByText("Agent session")).not.toBeInTheDocument();
  expect(screen.queryByText("/Users/name/project", { exact: true })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Copy terminal output" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Run terminal command" })).not.toBeInTheDocument();
  expect(screen.queryByText(/background work/i)).not.toBeInTheDocument();

  rerender(<TerminalDrawer open height={132} response={response} onClose={() => undefined} />);

  expect(screen.getByRole("complementary", { name: "Terminal" })).toHaveStyle({ height: "260px" });
  expect(screen.getByText("ok")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Copy terminal output" })).toBeInTheDocument();
});

test("renders a pending terminal approval with command and cwd before execution", () => {
  const onApprove = vi.fn();
  const onDeny = vi.fn();
  render(
    <TerminalDrawer
      open
      height={260}
      response={{
        workspaceId: "workspace.desktoplab",
        terminalId: "terminal.local",
        state: "approval_required",
        command: "npm test",
        cwd: "apps/desktop",
        approval: {
          approvalId: "approval.terminal.local",
          state: "pending",
          copy: "Terminal command `npm test` in `apps/desktop` requires approval.",
        },
        events: [],
      }}
      onApprove={onApprove}
      onDeny={onDeny}
      onClose={() => undefined}
    />,
  );

  expect(screen.getByRole("complementary", { name: "Terminal" })).toBeInTheDocument();
  expect(screen.queryByText("Approval required")).not.toBeInTheDocument();
  expect(screen.getByText("desktop")).toBeInTheDocument();
  expect(screen.getByText("desktop % npm test")).toBeInTheDocument();
  expect(screen.getByText(/Terminal command `npm test` in `apps\/desktop` requires approval\./)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Approve command" }));
  fireEvent.click(screen.getByRole("button", { name: "Deny command" }));

  expect(onApprove).toHaveBeenCalledWith("approval.terminal.local");
  expect(onDeny).toHaveBeenCalledWith("approval.terminal.local");
});

test("accepts typed terminal commands and keeps shift enter inside the command field", async () => {
  const onRunCommand = vi.fn().mockResolvedValue({
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "npm test",
    cwd: ".",
    approval: {
      approvalId: "",
      state: "approved",
      copy: "",
    },
    events: [{
      eventId: "terminal.local.output",
      kind: "output",
      status: "exited",
      exitCode: 0,
      stdout: "ok",
      stderr: "",
      stdoutTruncated: false,
      redacted: false,
    }],
  } satisfies TerminalCommandResponse);

  render(
    <TerminalDrawer
      open
      height={260}
      workspacePath="/Users/name/project"
      sessionLabel="Agent session"
      onRunCommand={onRunCommand}
      onClose={() => undefined}
    />,
  );

  const input = screen.getByRole("textbox", { name: "Terminal input" });
  fireEvent.change(input, { target: { value: "npm" } });
  fireEvent.keyDown(input, { key: "Enter", shiftKey: true });

  expect(onRunCommand).not.toHaveBeenCalled();
  expect(input).toHaveValue("npm");

  fireEvent.change(input, { target: { value: "npm test" } });
  fireEvent.keyDown(input, { key: "Enter" });

  expect(onRunCommand).toHaveBeenCalledWith({
    command: "npm test",
    cwd: ".",
  });
  expect(await screen.findByText("project % npm test")).toBeInTheDocument();
  expect(screen.queryByText("Approval required")).not.toBeInTheDocument();
  expect(input).toHaveValue("");
});

test("places the terminal command line inside the terminal surface like a normal shell", () => {
  render(
    <TerminalDrawer
      open
      height={260}
      workspacePath="/Users/name/project"
      sessionLabel="Agent session"
      onRunCommand={vi.fn()}
      onClose={() => undefined}
    />,
  );

  const commandLine = screen.getByTestId("terminal-command-line");
  const scrollRegion = screen.getByTestId("terminal-scroll-region");

  expect(commandLine).toContainElement(screen.getByRole("textbox", { name: "Terminal input" }));
  expect(commandLine).not.toHaveClass("border-b");
  expect(scrollRegion).toContainElement(commandLine);
  expect(screen.getByText("project %")).toBeInTheDocument();
  expect(screen.queryByText("Agent session")).not.toBeInTheDocument();
});

test("opens additional workspace terminal tabs and keeps commands scoped to the repository", async () => {
  const onRunCommand = vi.fn().mockResolvedValue({
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local.2",
    state: "completed",
    command: "pwd",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [],
  } satisfies TerminalCommandResponse);

  render(<TerminalDrawer open height={260} workspacePath="/Users/name/project" onRunCommand={onRunCommand} onClose={() => undefined} />);

  expect(screen.getByRole("tab", { name: "project" })).toHaveAttribute("aria-selected", "true");
  fireEvent.click(screen.getByRole("button", { name: "Open workspace terminal" }));

  expect(screen.getByRole("tab", { name: "project" })).toHaveAttribute("aria-selected", "false");
  expect(screen.getByRole("tab", { name: "project 2" })).toHaveAttribute("aria-selected", "true");

  const input = screen.getByRole("textbox", { name: "Terminal input" });
  fireEvent.change(input, { target: { value: "pwd" } });
  fireEvent.keyDown(input, { key: "Enter" });

  expect(onRunCommand).toHaveBeenCalledWith({
    command: "pwd",
    cwd: ".",
  });
  expect(await screen.findByText("project % pwd")).toBeInTheDocument();
});

test("terminal input only exposes cursor chrome instead of composer focus styling", () => {
  render(<TerminalDrawer open height={260} workspacePath="/Users/name/project" onRunCommand={vi.fn()} onClose={() => undefined} />);

  const input = screen.getByRole("textbox", { name: "Terminal input" });

  expect(input).toHaveClass("caret-ink");
  expect(input).toHaveClass("focus:ring-0");
  expect(input).not.toHaveClass("rounded-desktop");
  expect(input).not.toHaveClass("border");
  expect(input).not.toHaveClass("bg-panel");
});

test("renders redacted terminal output events", () => {
  const response: TerminalCommandResponse = {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "printf ok",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [
      {
        eventId: "terminal.local.output",
        kind: "output",
        stdout: "token=<redacted>",
        stderr: "",
        status: "exited",
        exitCode: 0,
        stdoutTruncated: false,
        redacted: true,
      },
    ],
  };

  render(<TerminalDrawer open height={260} response={response} onClose={() => undefined} />);

  expect(screen.getAllByText("Completed")).toHaveLength(2);
  expect(screen.getByText("Workspace shell % printf ok")).toBeInTheDocument();
  expect(screen.getByText("token=<redacted>")).toBeInTheDocument();
  expect(screen.getByText("Redacted")).toBeInTheDocument();
});

test("marks response-backed commands as agent session commands", () => {
  const response: TerminalCommandResponse = {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "npm test",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [],
  };

  render(<TerminalDrawer open height={260} response={response} sessionLabel="Agent session" onClose={() => undefined} />);

  expect(screen.getByText("Agent session command")).toBeInTheDocument();
  expect(screen.getByText("Workspace shell % npm test")).toBeInTheDocument();
});

test("denied command never shows output as if it ran", () => {
  render(
    <TerminalDrawer
      open
      height={260}
      response={{
        workspaceId: "workspace.desktoplab",
        terminalId: "terminal.local",
        state: "denied",
        command: "rm -rf target",
        cwd: ".",
        approval: {
          approvalId: "approval.terminal.local",
          state: "denied",
          copy: "Command denied.",
        },
        events: [],
      }}
      onClose={() => undefined}
    />,
  );

  expect(screen.getByText("Denied")).toBeInTheDocument();
  expect(screen.getByText("Command denied.")).toBeInTheDocument();
  expect(screen.queryByText("No output")).not.toBeInTheDocument();
});

test("failed terminal events use user-facing status copy", () => {
  render(
    <TerminalDrawer
      open
      height={260}
      response={{
        workspaceId: "workspace.desktoplab",
        terminalId: "terminal.local",
        state: "completed",
        command: "missing-command",
        cwd: ".",
        approval: {
          approvalId: "approval.terminal.local",
          state: "approved",
          copy: "Approved",
        },
        events: [
          {
            eventId: "terminal.local.output",
            kind: "output",
            stdout: "",
            stderr: "command not found",
            status: "failed_to_spawn",
            exitCode: null,
            stdoutTruncated: false,
            redacted: false,
          },
        ],
      }}
      onClose={() => undefined}
    />,
  );

  expect(screen.getByText("Failed to start")).toBeInTheDocument();
  expect(screen.queryByText("failed_to_spawn")).not.toBeInTheDocument();
});

test("ingests terminal replay frames without duplicating rendered event lines", () => {
  const response: TerminalCommandResponse = {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "npm test",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [
      {
        eventId: "event.1",
        kind: "output",
        stdout: "cached output",
        stderr: "",
        status: "exited",
        exitCode: 0,
        stdoutTruncated: false,
        redacted: false,
      },
    ],
  };
  const eventFrames: BackendEventFrame[] = [
    terminalFrame(7, "event.1", "cached output"),
    terminalFrame(8, "event.2", "fresh output"),
  ];

  render(<TerminalDrawer open height={260} response={response} eventFrames={eventFrames} onClose={() => undefined} />);

  expect(screen.getAllByText("cached output")).toHaveLength(1);
  expect(screen.getByText("fresh output")).toBeInTheDocument();
});

test("does not merge replay events into a locally executed terminal response", async () => {
  const response: TerminalCommandResponse = {
    workspaceId: "workspace.desktoplab",
    terminalId: "terminal.local",
    state: "completed",
    command: "printf local-output",
    cwd: ".",
    approval: {
      approvalId: "approval.terminal.local",
      state: "approved",
      copy: "Approved",
    },
    events: [
      {
        eventId: "event.response",
        kind: "output",
        stdout: "local-output",
        stderr: "",
        status: "exited",
        exitCode: 0,
        stdoutTruncated: false,
        redacted: false,
      },
    ],
  };
  const eventFrames = [terminalFrame(7, "event.replay", "local-output")];

  render(
    <TerminalDrawer
      open
      height={260}
      workspacePath="/Users/name/project"
      eventFrames={eventFrames}
      onRunCommand={async () => response}
      onClose={() => undefined}
    />,
  );

  fireEvent.change(screen.getByRole("textbox", { name: "Terminal input" }), {
    target: { value: "printf local-output" },
  });
  fireEvent.keyDown(screen.getByRole("textbox", { name: "Terminal input" }), { key: "Enter" });

  expect(await screen.findAllByText("local-output", { exact: true })).toHaveLength(1);
});

function terminalFrame(sequence: number, eventId: string, stdout: string): BackendEventFrame {
  return {
    sequence,
    scope: "terminal",
    payload: JSON.stringify({
      terminalId: "terminal.local",
      eventId,
      kind: "terminal.output",
      stdout,
      stderr: "",
      status: "exited",
      exitCode: 0,
      stdoutTruncated: false,
      redacted: false,
    }),
  };
}
