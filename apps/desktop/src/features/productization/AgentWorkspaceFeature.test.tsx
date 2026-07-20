// @vitest-environment jsdom
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { AppProviders } from "../../app/AppProviders";
import type { DesktopLabApiClient } from "../../api/client";
import type { AgentWorkspaceSnapshot, ApprovalSummary, GitOperationsSnapshot, SessionControlResponse } from "../../api/types";
import { AgentWorkspaceFeature } from "./AgentWorkspaceFeature";

test("renders thread pages as conversation plus fixed composer only", async () => {
  renderAgentWorkspace();

  expect(await screen.findByTestId("agent-thread-surface")).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Conversation" })).toHaveClass("sr-only");
  expect(screen.getByTestId("agent-thread-surface")).toHaveClass("h-full");
  expect(screen.getByTestId("agent-thread-surface")).toHaveClass("min-h-0");
  expect(screen.getByTestId("agent-conversation-scroll-region")).toHaveClass("overflow-auto");
  expect(screen.getByTestId("agent-conversation-scroll-region")).toHaveClass("flex-1");
  expect(screen.getByTestId("agent-composer")).toHaveClass("shrink-0");
  expect(screen.getByTestId("agent-composer").parentElement).toHaveAttribute("data-testid", "agent-thread-surface");
  expect(screen.getByTestId("agent-conversation-scroll-region")).not.toContainElement(screen.getByTestId("agent-composer"));
  expect(screen.getByTestId("agent-composer")).not.toHaveClass("sticky");
  expect(screen.getByRole("textbox", { name: "Prompt" })).toBeInTheDocument();
  expect(screen.getByPlaceholderText("Ask DesktopLab to work on this repository")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Attach external files" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Approval: Ask for approval" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Model: Local" })).not.toBeInTheDocument();
  expect(screen.queryByText("Local runner")).not.toBeInTheDocument();
  expect(screen.queryByText("Runs on this machine")).not.toBeInTheDocument();
  expect(screen.queryByText("Repository context")).not.toBeInTheDocument();
  expect(screen.queryByText("Next review")).not.toBeInTheDocument();
  expect(screen.queryByText("backend.ollama")).not.toBeInTheDocument();
  expect(screen.getAllByText("Read repository").length).toBeGreaterThan(0);
  expect(screen.queryByText("Terminal command")).not.toBeInTheDocument();
  expect(screen.queryByText("Tests passed")).not.toBeInTheDocument();
  expect(screen.getAllByText("2 files changed").length).toBeGreaterThan(0);
  expect(screen.queryByLabelText("Workbench readiness")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Pause session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Resume session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Cancel session" })).not.toBeInTheDocument();
});

test("composer command bar exposes compact route controls without trust copy", async () => {
  renderAgentWorkspace();

  await screen.findByTestId("agent-thread-surface");

  const sendButtons = screen.getAllByRole("button", { name: "Send prompt" });
  expect(sendButtons).toHaveLength(1);
  expect(sendButtons[0]).toBeDisabled();
  expect(sendButtons[0]).toHaveAttribute("title", "Enter a prompt to send.");
  expect(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" })).toHaveTextContent("Qwen Coder 7B");
  expect(screen.queryByText("Local route. Model work runs on this computer.")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Attach external files" })).toHaveClass("h-8");
  expect(screen.getByRole("button", { name: "Approval: Ask for approval" })).toHaveClass("h-8");
  expect(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" })).toHaveClass("h-8");
});

test("composer prompt keeps visible text and caret colors", async () => {
  renderAgentWorkspace();

  await screen.findByTestId("agent-thread-surface");
  const prompt = screen.getByRole("textbox", { name: "Prompt" });

  expect(prompt).toHaveClass("text-ink");
  expect(prompt).toHaveClass("caret-ink");
});

test("composer submits the current prompt with Enter", async () => {
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Leggi il file" } });
  fireEvent.keyDown(screen.getByRole("textbox", { name: "Prompt" }), { key: "Enter", code: "Enter" });

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith(expect.objectContaining({ initialPrompt: "Leggi il file" })),
  );
});

test("composer accepts Return and numpad Enter as submit keys", async () => {
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  const prompt = screen.getByRole("textbox", { name: "Prompt" });

  fireEvent.change(prompt, { target: { value: "Return prompt" } });
  fireEvent.keyDown(prompt, { key: "Return", code: "Enter" });
  await waitFor(() => expect(createSession).toHaveBeenCalledWith(expect.objectContaining({ initialPrompt: "Return prompt" })));
  expect(createSession).toHaveBeenCalledTimes(1);

  createSession.mockClear();
  fireEvent.change(prompt, { target: { value: "Numpad prompt" } });
  fireEvent.keyDown(prompt, { key: "Enter", code: "NumpadEnter" });
  await waitFor(() => expect(createSession).toHaveBeenCalledWith(expect.objectContaining({ initialPrompt: "Numpad prompt" })));
  expect(createSession).toHaveBeenCalledTimes(1);
});

test("session changes do not open an automatic corner panel in the thread", async () => {
  const onOpenChanges = vi.fn();
  renderAgentWorkspace({}, { onOpenChanges });

  await screen.findByTestId("agent-thread-surface");

  const composer = screen.getByTestId("agent-composer");
  expect(within(composer).queryByRole("button", { name: "Changes" })).not.toBeInTheDocument();
  expect(screen.queryByRole("complementary", { name: "Session changes" })).not.toBeInTheDocument();
  expect(onOpenChanges).not.toHaveBeenCalled();
});

test("composer approval menu updates the backend-owned session mode", async () => {
  const updateSessionApprovalMode = vi.fn().mockResolvedValue(approvalModes("full_access"));
  renderAgentWorkspace({ updateSessionApprovalMode });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Approval: Ask for approval" }));

  expect(screen.getByRole("menuitemradio", { name: "Ask for approval" })).toBeChecked();
  expect(screen.getByRole("menuitemradio", { name: "Approve routine actions" })).toBeInTheDocument();
  expect(screen.getByRole("menuitemradio", { name: "Allow workspace writes" })).toBeInTheDocument();
  expect(screen.getByRole("menuitemradio", { name: "Full local access" })).toBeInTheDocument();
  expect(screen.getByText("Recommended for small local models and first-time setup.")).toBeInTheDocument();
  expect(screen.getByText("Workspace file writes can continue while commands and git actions still stop.")).toBeInTheDocument();
  expect(screen.getByText("External providers and protected data still stop for you.")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("menuitemradio", { name: "Full local access" }));

  await waitFor(() => expect(updateSessionApprovalMode).toHaveBeenCalledWith({ mode: "full_access" }));
});

test("composer route menu updates backend-owned route selection", async () => {
  const updateRouteSelection = vi.fn().mockResolvedValue(routeOptions());
  renderAgentWorkspace({ updateRouteSelection });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" }));

  expect(screen.getByRole("menuitemradio", { name: "Qwen Coder 7B · Ollama" })).toBeChecked();
  expect(screen.getByRole("menuitemradio", { name: "DeepSeek Coder 7B · Ollama" })).toBeEnabled();
  expect(screen.queryByRole("menuitemradio", { name: "OpenAI GPT-4.1 · Cloud" })).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("menuitemradio", { name: "DeepSeek Coder 7B · Ollama" }));

  await waitFor(() => expect(updateRouteSelection).toHaveBeenCalledWith({ routeId: "route.local.deepseek-coder-7b-q4" }));
});

test("composer model control shows the installed model even when it is the only choice", async () => {
  renderAgentWorkspace({
    routeOptions: vi.fn().mockResolvedValue({
      selectedRouteId: "route.local.qwen-coder-7b",
      options: [routeOptions().options[0]],
    }),
  });

  await screen.findByTestId("agent-thread-surface");

  const model = screen.getByRole("button", { name: "Selected model Qwen Coder 7B" });
  expect(model).toBeEnabled();
  expect(model).toHaveTextContent("Qwen Coder 7B");
  fireEvent.click(model);
  expect(screen.getByRole("menuitemradio", { name: "Qwen Coder 7B · Ollama" })).toBeChecked();
});

test("composer model menu excludes cloud and bridge routes", async () => {
  const updateRouteSelection = vi.fn().mockResolvedValue(routeOptionsWithBlockedAlternatives());
  renderAgentWorkspace({
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithBlockedAlternatives()),
    updateRouteSelection,
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" }));

  expect(screen.getByRole("menuitemradio", { name: "Qwen Coder 7B · Ollama" })).toBeChecked();
  expect(screen.queryByRole("menuitemradio", { name: "OpenAI GPT-4.1 · Cloud" })).not.toBeInTheDocument();
  expect(screen.queryByRole("menuitemradio", { name: "Codex bridge · External" })).not.toBeInTheDocument();
  expect(updateRouteSelection).not.toHaveBeenCalled();
});

test("composer route menu hides local catalog entries that are not execution ready", async () => {
  renderAgentWorkspace({
    routeOptions: vi.fn().mockResolvedValue({
      selectedRouteId: "route.local.qwen-coder-7b",
      options: [
        ...routeOptions().options,
        {
          routeId: "route.local.nemotron-70b-q4",
          backendId: "backend.ollama",
          backendKind: "local",
          label: "NVIDIA Nemotron 70B · Ollama",
          modelId: "model.nemotron-70b-q4",
          runtimeId: "runtime.ollama",
          executionBackendId: "backend.ollama",
          modelDisplayName: "NVIDIA Nemotron 70B",
          runtimeDisplayName: "Ollama",
          status: "unavailable",
          disabledReason: "This model is not ready on this computer.",
        },
      ],
    }),
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" }));

  expect(screen.queryByRole("menuitemradio", { name: "NVIDIA Nemotron 70B · Ollama" })).not.toBeInTheDocument();
});

test("composer model menu keeps an available codex bridge in provider settings", async () => {
  const updateRouteSelection = vi.fn().mockResolvedValue(routeOptionsWithCodexBridge());
  renderAgentWorkspace({
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithCodexBridge()),
    updateRouteSelection,
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" }));

  expect(screen.queryByRole("menuitemradio", { name: "Codex bridge · External" })).not.toBeInTheDocument();
  expect(updateRouteSelection).not.toHaveBeenCalled();
});

test("composer model menu does not mix external egress controls with installed models", async () => {
  renderAgentWorkspace({
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithCodexBridge()),
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Selected model Qwen Coder 7B" }));

  expect(screen.queryByRole("menuitemradio", { name: "Codex bridge · External" })).not.toBeInTheDocument();
});

test("composer requires explicit egress approval before sending attached context to codex", async () => {
  const createApproval = vi.fn().mockResolvedValue({
    approvalId: "approval.egress.1",
    sessionId: "session.pending",
    action: "provider.egress",
    operationId: "provider.openai:route.external.codex:workspace.desktoplab",
    state: "pending",
  });
  const resolveApproval = vi.fn().mockResolvedValue({ approvalId: "approval.egress.1", state: "approved" });
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession(agentSnapshotWithCodexRoute())),
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithCodexBridge("route.external.codex")),
    createApproval,
    resolveApproval,
    createSession,
  });

  await screen.findByTestId("agent-thread-surface");
  const input = screen.getByLabelText("Choose external files") as HTMLInputElement;
  fireEvent.change(input, {
    target: {
      files: [new File(["notes"], "brief.txt", { type: "text/plain" })],
    },
  });
  await screen.findByRole("button", { name: "Attach external files, 1 attached" });

  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Explain this file" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(createApproval).toHaveBeenCalledWith({
      sessionId: "session.pending",
      action: "provider.egress",
      operationId: "provider.openai:route.external.codex:workspace.desktoplab",
      payload: {
        providerId: "provider.openai",
        routeId: "route.external.codex",
        backendId: "backend.codex",
        workspaceId: "workspace.desktoplab",
        initialPrompt: "Explain this file",
        contextPaths: [],
        externalAttachments: [
          {
            name: "brief.txt",
            size: 5,
            mediaType: "text/plain",
            contentAttached: true,
            contentSha256: "sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309",
            truncated: false,
          },
        ],
      },
    }),
  );
  expect(createSession).not.toHaveBeenCalled();
  expect(screen.getByRole("group", { name: "External route approval" })).toHaveTextContent("Send attached context to the external route?");

  fireEvent.click(screen.getByRole("button", { name: "Approve" }));

  await waitFor(() => expect(resolveApproval).toHaveBeenCalledWith("approval.egress.1", { resolution: "approve" }));
  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith({
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.codex",
      initialPrompt: "Explain this file",
      externalAttachments: [
        {
          name: "brief.txt",
          size: 5,
          mediaType: "text/plain",
          contentText: "notes",
          contentSha256: "sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309",
          truncated: false,
        },
      ],
      approvalId: "approval.egress.1",
    }),
  );
});

test("composer can deny external egress without starting the session", async () => {
  const createApproval = vi.fn().mockResolvedValue({
    approvalId: "approval.egress.2",
    sessionId: "session.pending",
    action: "provider.egress",
    operationId: "provider.openai:route.external.codex:workspace.desktoplab",
    state: "pending",
  });
  const resolveApproval = vi.fn().mockResolvedValue({ approvalId: "approval.egress.2", state: "denied" });
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession(agentSnapshotWithCodexRoute())),
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithCodexBridge("route.external.codex")),
    createApproval,
    resolveApproval,
    createSession,
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByLabelText("Choose external files"), {
    target: { files: [new File(["notes"], "brief.txt", { type: "text/plain" })] },
  });
  await screen.findByRole("button", { name: "Attach external files, 1 attached" });
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Explain this file" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));
  await screen.findByRole("group", { name: "External route approval" });

  fireEvent.click(screen.getByRole("button", { name: "Deny" }));

  await waitFor(() => expect(resolveApproval).toHaveBeenCalledWith("approval.egress.2", { resolution: "deny" }));
  expect(createSession).not.toHaveBeenCalled();
  expect(screen.getByRole("textbox", { name: "Prompt" })).toHaveValue("Explain this file");
});

test("blocked filesystem writes expose inline approval controls in the active thread", async () => {
  const resolveApproval = vi.fn().mockResolvedValue({ approvalId: "approval.write.1", state: "approved" });
  const continueSession = vi.fn().mockResolvedValue({ ...agentSnapshot().session!, summary: "Approved write completed" });
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithApproval()),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [writeApproval()] }),
    resolveApproval,
    continueSession,
  });

  await screen.findByTestId("agent-thread-surface");
  expect(screen.getByRole("group", { name: "Thread approval required" })).toHaveTextContent("Write DESKTOPLAB_AGENT_NOTES.md");
  expect(screen.getByRole("button", { name: "Approve" })).toHaveClass("text-canvas");
  expect(screen.getByRole("button", { name: "Approve" })).not.toHaveClass("text-paper");

  fireEvent.click(screen.getByRole("button", { name: "Approve" }));

  await waitFor(() => expect(screen.queryByRole("group", { name: "Thread approval required" })).not.toBeInTheDocument());
  await waitFor(() => expect(resolveApproval).toHaveBeenCalledWith("approval.write.1", { resolution: "approve" }));
  expect(continueSession).not.toHaveBeenCalled();
});

test("failed thread approval remains visible and reports the error", async () => {
  const resolveApproval = vi.fn().mockRejectedValue(new Error("local API unavailable"));
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithApproval()),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [writeApproval()] }),
    resolveApproval,
  });

  await screen.findByRole("group", { name: "Thread approval required" });
  fireEvent.click(screen.getByRole("button", { name: "Approve" }));

  expect(await screen.findByRole("alert")).toHaveTextContent("Approval was not saved. Try again.");
  expect(screen.getByRole("group", { name: "Thread approval required" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Approve" })).toBeEnabled();
});

test("retrying a failed thread approval clears the stale error while resolving", async () => {
  let resolveRetry: ((value: { approvalId: string; state: string }) => void) | undefined;
  const resolveApproval = vi
    .fn()
    .mockRejectedValueOnce(new Error("local API unavailable"))
    .mockImplementationOnce(() => new Promise((resolve) => { resolveRetry = resolve; }));
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithApproval()),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [writeApproval()] }),
    resolveApproval,
  });

  await screen.findByRole("group", { name: "Thread approval required" });
  fireEvent.click(screen.getByRole("button", { name: "Approve" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Approval was not saved. Try again.");

  fireEvent.click(screen.getByRole("button", { name: "Approve" }));
  await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  expect(screen.getByRole("button", { name: "Approve" })).toBeDisabled();
  await act(async () => {
    resolveRetry?.({ approvalId: "approval.write.1", state: "approved" });
  });
});

test("blocked filesystem writes expose inline approval controls from session payload", async () => {
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshotWithApproval(),
      session: {
        ...agentSnapshotWithApproval().session!,
        pendingApprovals: [writeApproval()],
      },
    } satisfies AgentWorkspaceSnapshot),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [] }),
  });

  await screen.findByTestId("agent-thread-surface");
  expect(screen.getByRole("group", { name: "Thread approval required" })).toHaveTextContent("Write DESKTOPLAB_AGENT_NOTES.md");
});

test("blocked terminal commands expose command cwd risk and reason inline", async () => {
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshot("blocked"),
      session: {
        ...agentSnapshot("blocked").session!,
        pendingApprovals: [terminalApproval()],
      },
    } satisfies AgentWorkspaceSnapshot),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [] }),
  });

  await screen.findByTestId("agent-thread-surface");
  const approval = screen.getByRole("group", { name: "Thread approval required" });
  expect(approval).toHaveTextContent("Run npm test");
  expect(approval).toHaveTextContent("Command");
  expect(approval).toHaveTextContent("npm test");
  expect(approval).toHaveTextContent("Cwd");
  expect(approval).toHaveTextContent("workspace");
  expect(approval).toHaveTextContent("Risk");
  expect(approval).toHaveTextContent("medium");
  expect(approval).toHaveTextContent("Reason");
});

test("blocked sends refetch thread approvals immediately", async () => {
  const listApprovals = vi
    .fn()
    .mockResolvedValueOnce({ approvals: [] })
    .mockResolvedValueOnce({ approvals: [writeApproval()] });
  const agentWorkspace = vi
    .fn()
    .mockResolvedValueOnce(agentSnapshot())
    .mockResolvedValueOnce(agentSnapshotWithApproval());
  const createSession = vi.fn().mockResolvedValue(agentSnapshotWithApproval().session);
  renderAgentWorkspace({ agentWorkspace, createSession, listApprovals });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Create notes" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  expect(await screen.findByRole("group", { name: "Thread approval required" })).toHaveTextContent("Write DESKTOPLAB_AGENT_NOTES.md");
  expect(listApprovals).toHaveBeenCalledTimes(2);
});

test("approval stays working until the backend-owned resolution completes", async () => {
  let finishResolve!: (approval: { approvalId: string; state: "approved" }) => void;
  const resolveApproval = vi.fn(
    () =>
      new Promise<{ approvalId: string; state: "approved" }>((resolve) => {
        finishResolve = resolve;
      }),
  );
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithApproval()),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [writeApproval()] }),
    resolveApproval,
  });

  await screen.findByRole("group", { name: "Thread approval required" });
  fireEvent.click(screen.getByRole("button", { name: "Approve" }));

  expect(await screen.findByText("Agent is working")).toBeInTheDocument();
  await act(async () => {
    finishResolve({ approvalId: "approval.write.1", state: "approved" });
  });
});

test("working indicator appears above the composer while a send is in progress", async () => {
  let resolveSession!: (session: NonNullable<AgentWorkspaceSnapshot["session"]>) => void;
  const createSession = vi.fn(
    () =>
      new Promise<NonNullable<AgentWorkspaceSnapshot["session"]>>((resolve) => {
        resolveSession = resolve;
      }),
  );
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Funzioni?" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  expect(await screen.findByText("Agent is working")).toBeInTheDocument();

  await act(async () => {
    resolveSession(agentSnapshot().session!);
  });
});

test("composer can return to the local route from an external egress prompt", async () => {
  const updateRouteSelection = vi.fn().mockResolvedValue(routeOptions());
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession(agentSnapshotWithCodexRoute())),
    routeOptions: vi.fn().mockResolvedValue(routeOptionsWithCodexBridge("route.external.codex")),
    createApproval: vi.fn().mockResolvedValue({
      approvalId: "approval.egress.3",
      sessionId: "session.pending",
      action: "provider.egress",
      operationId: "provider.openai:route.external.codex:workspace.desktoplab",
      state: "pending",
    }),
    updateRouteSelection,
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByLabelText("Choose external files"), {
    target: { files: [new File(["notes"], "brief.txt", { type: "text/plain" })] },
  });
  await screen.findByRole("button", { name: "Attach external files, 1 attached" });
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Explain this file" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));
  await screen.findByRole("group", { name: "External route approval" });

  fireEvent.click(screen.getByRole("button", { name: "Use local" }));

  await waitFor(() => expect(updateRouteSelection).toHaveBeenCalledWith({ routeId: "route.local.qwen-coder-7b" }));
});

test("composer paperclip uses external file attachments instead of repository context menu", async () => {
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  expect(screen.queryByRole("menu", { name: "Attach context" })).not.toBeInTheDocument();
  expect(screen.queryByRole("menuitemcheckbox", { name: "README.md" })).not.toBeInTheDocument();
  const input = screen.getByLabelText("Choose external files") as HTMLInputElement;
  expect(input.type).toBe("file");
  expect(input.multiple).toBe(true);
  expect(input.accept).toContain("text/*");

  fireEvent.change(input, {
    target: {
      files: [new File(["notes"], "brief.txt", { type: "text/plain" })],
    },
  });
  expect(await screen.findByRole("button", { name: "Attach external files, 1 attached" })).toBeInTheDocument();

  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Explain this file" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith(
      expect.objectContaining({
        initialPrompt: "Explain this file",
        externalAttachments: [
          {
            name: "brief.txt",
            size: 5,
            mediaType: "text/plain",
            contentText: "notes",
            contentSha256: "sha256:ab5aa97074c454a0632057e704220d9a6678fbf773a0a5806fc09b8173b07309",
            truncated: false,
          },
        ],
      }),
    ),
  );
});

test("composer refuses binary attachments instead of pretending the model received them", async () => {
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()) });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByLabelText("Choose external files"), {
    target: { files: [new File([new Uint8Array([137, 80, 78, 71])], "screen.png", { type: "image/png" })] },
  });

  expect(await screen.findByRole("status")).toHaveTextContent("Only text and source files can be attached.");
  expect(screen.queryByRole("button", { name: "Attach external files, 1 attached" })).not.toBeInTheDocument();
});

test("composer preserves external attachments when session creation fails", async () => {
  const createSession = vi.fn().mockRejectedValue(new Error("backend unavailable"));
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  const input = screen.getByLabelText("Choose external files") as HTMLInputElement;
  fireEvent.change(input, {
    target: {
      files: [new File(["notes"], "brief.txt", { type: "text/plain" })],
    },
  });
  expect(await screen.findByRole("button", { name: "Attach external files, 1 attached" })).toBeInTheDocument();

  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Explain this file" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() => expect(createSession).toHaveBeenCalled());
  await waitFor(() => expect(screen.getByRole("textbox", { name: "Prompt" })).toHaveValue("Explain this file"));
  expect(screen.getByRole("button", { name: "Attach external files, 1 attached" })).toBeInTheDocument();
});

test("workbench keeps setup metadata out of the thread composer surface", async () => {
  renderAgentWorkspace();

  await screen.findByTestId("agent-thread-surface");

  expect(screen.queryByLabelText("Workbench readiness")).not.toBeInTheDocument();
  expect(screen.queryByText("Approvals: Ask before writes")).not.toBeInTheDocument();
  expect(screen.queryByText("backend.ollama")).not.toBeInTheDocument();
  expect(screen.queryByText("runtime_not_ready")).not.toBeInTheDocument();
});

test("stop button cancels a running backend-owned session", async () => {
  const sessionControl = vi.fn().mockResolvedValue({ accepted: true } satisfies SessionControlResponse);
  renderAgentWorkspace({ sessionControl, agentWorkspace: vi.fn().mockResolvedValue(agentSnapshot("running")) });

  await screen.findByTestId("agent-thread-surface");
  expect(screen.getByRole("button", { name: "Stop agent" })).toBeEnabled();
  expect(screen.queryByRole("button", { name: "Pause session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Resume session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Cancel session" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop agent" }));

  await waitFor(() => expect(sessionControl).toHaveBeenCalledWith("session.1", { action: "cancel" }));
});

test("blocked routes disable agent start without composer session controls", async () => {
  const onOpenSetup = vi.fn();
  renderAgentWorkspace(
    {
      agentWorkspace: vi.fn().mockResolvedValue({
        ...agentSnapshot(),
        route: {
          status: "blocked",
          backendDisplayName: "Cloud model",
          backendKind: "cloud",
          summary: "A local coding model must finish downloading first.",
          reasons: ["Qwen Coder is not ready yet."],
          nextAction: "complete_setup",
          nextActionLabel: "Finish setup",
          requiredCapabilities: ["Chat"],
          needsFallbackApproval: true,
        },
      } satisfies AgentWorkspaceSnapshot),
    },
    { onOpenSetup },
  );

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Finish setup" }));
  expect(onOpenSetup).toHaveBeenCalledTimes(1);
  expect(screen.getByText("A local coding model must finish downloading first.")).toBeInTheDocument();
  expect(screen.getByText("Qwen Coder is not ready yet.")).toBeInTheDocument();
  expect(screen.getByText("Next: Finish setup")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Pause session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Resume session" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Cancel session" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Selected model Finish setup" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Blocked/ })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Send prompt" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Send prompt" })).toHaveAttribute("title", "Finish setup before sending a prompt.");
});

test("submits prompt through the real create session boundary", async () => {
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  expect(screen.getByRole("button", { name: "Send prompt" })).toBeDisabled();

  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Implement provider routing tests" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith({
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      initialPrompt: "Implement provider routing tests",
    }),
  );
  expect(createSession).toHaveBeenCalledTimes(1);
});

test("subsequent prompts continue the selected thread instead of creating a new thread", async () => {
  const createSession = vi.fn();
  const selectedSession = {
    ...agentSnapshot().session!,
    timeline: [
      { sequence: 1, kind: "planning", message: "First prompt", createdAt: "2026-07-01T08:00:00Z" },
      { sequence: 2, kind: "assistant", message: "First response", createdAt: "2026-07-01T08:01:00Z" },
    ],
  };
  const continueSession = vi.fn().mockResolvedValue({
    ...selectedSession,
    timeline: [
      { sequence: 1, kind: "planning", message: "First prompt", createdAt: "2026-07-01T08:00:00Z" },
      { sequence: 2, kind: "assistant", message: "First response", createdAt: "2026-07-01T08:01:00Z" },
      { sequence: 3, kind: "planning", message: "Second prompt", createdAt: "2026-07-01T08:02:00Z" },
      { sequence: 4, kind: "assistant", message: "Second response", createdAt: "2026-07-01T08:03:00Z" },
    ],
  });
  const onSessionStarted = vi.fn();
  renderAgentWorkspace(
    {
      agentWorkspace: vi.fn().mockResolvedValue({ ...agentSnapshot(), session: selectedSession }),
      createSession,
      continueSession,
    },
    {
      onSessionStarted,
      selectedSession,
    },
  );

  await screen.findByText("First response");
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Second prompt" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(continueSession).toHaveBeenCalledWith("session.1", {
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      prompt: "Second prompt",
    }),
  );
  expect(createSession).not.toHaveBeenCalled();
  await waitFor(() => expect(onSessionStarted).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "session.1" })));
});

test("control-plane terminal state replaces a stale selected drawer snapshot", async () => {
  const selectedSession = {
    ...agentSnapshot().session!,
    state: "blocked" as const,
    timeline: [{ sequence: 1, kind: "planning", message: "Inspect repository", createdAt: "2026-07-01T08:00:00Z" }],
  };
  const failedSession = {
    ...selectedSession,
    state: "failed" as const,
    timeline: [
      ...selectedSession.timeline,
      { sequence: 2, kind: "failed", message: "local_inference_failed", createdAt: "2026-07-01T08:00:01Z" },
    ],
  };
  renderAgentWorkspace(
    { agentWorkspace: vi.fn().mockResolvedValue({ ...agentSnapshot(), session: failedSession }) },
    { selectedSession },
  );

  expect(await screen.findByText("Local inference failed before the agent could continue.")).toBeInTheDocument();
  expect(screen.queryByText("Agent loop is waiting for the next decision.")).not.toBeInTheDocument();
});

test("subsequent prompts continue the backend visible thread when no drawer thread is selected", async () => {
  const createSession = vi.fn();
  const continueSession = vi.fn().mockResolvedValue({
    ...agentSnapshot().session!,
    timeline: [
      { sequence: 1, kind: "planning", message: "First prompt", createdAt: "2026-07-01T08:00:00Z" },
      { sequence: 2, kind: "assistant", message: "First response", createdAt: "2026-07-01T08:01:00Z" },
      { sequence: 3, kind: "planning", message: "Second prompt", createdAt: "2026-07-01T08:02:00Z" },
      { sequence: 4, kind: "assistant", message: "Second response", createdAt: "2026-07-01T08:03:00Z" },
    ],
  });
  renderAgentWorkspace({ createSession, continueSession });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Second prompt" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(continueSession).toHaveBeenCalledWith("session.1", {
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      prompt: "Second prompt",
    }),
  );
  expect(createSession).not.toHaveBeenCalled();
});

test("composer sends with enter clears input and shows working state", async () => {
  let resolveSession!: (session: NonNullable<AgentWorkspaceSnapshot["session"]>) => void;
  const createSession = vi.fn(
    () =>
      new Promise<NonNullable<AgentWorkspaceSnapshot["session"]>>((resolve) => {
        resolveSession = resolve;
      }),
  );
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  const prompt = screen.getByRole("textbox", { name: "Prompt" });
  prompt.focus();
  fireEvent.change(prompt, { target: { value: "Funzioni?" } });
  await waitFor(() => expect(prompt).toHaveValue("Funzioni?"));
  fireEvent.keyDown(prompt, { key: "Enter", code: "Enter", charCode: 13, keyCode: 13 });

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith({
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      initialPrompt: "Funzioni?",
    }),
  );
  expect(prompt).toHaveValue("");
  expect(screen.queryByText("DesktopLab is working...")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Working" })).toBeDisabled();

  await act(async () => {
    resolveSession(agentSnapshot().session!);
  });
});

test("composer keeps shift enter as a newline gesture without submitting", async () => {
  const createSession = vi.fn().mockResolvedValue(agentSnapshot().session);
  renderAgentWorkspace({ agentWorkspace: vi.fn().mockResolvedValue(agentSnapshotWithoutSession()), createSession });

  await screen.findByTestId("agent-thread-surface");
  const prompt = screen.getByRole("textbox", { name: "Prompt" });
  prompt.focus();
  fireEvent.change(prompt, { target: { value: "Line one" } });
  await waitFor(() => expect(prompt).toHaveValue("Line one"));
  fireEvent.keyDown(prompt, { key: "Enter", code: "Enter", charCode: 13, keyCode: 13, shiftKey: true });

  expect(createSession).not.toHaveBeenCalled();
  expect(prompt).toHaveValue("Line one");
});

test("empty agent thread is prompt first after a workspace exists", async () => {
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshot(),
      session: null,
    } satisfies AgentWorkspaceSnapshot),
  });

  await screen.findByTestId("agent-thread-surface");

  expect(screen.getByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Explain this project" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Review changes" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Find setup instructions" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Prepare a task plan" })).toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Prompt" })).toBeInTheDocument();
  expect(screen.queryByText("Open a repository and ask DesktopLab to work on it.")).not.toBeInTheDocument();
});

test("draft thread mode suppresses the backend latest session until a prompt starts", async () => {
  const createSession = vi.fn().mockResolvedValue({
    ...agentSnapshot().session!,
    sessionId: "session.new",
    plan: "New prompt",
    timeline: [{ sequence: 1, kind: "user", message: "Start a fresh pass", createdAt: "2026-06-30T08:00:00Z" }],
  });
  const onSessionStarted = vi.fn();
  renderAgentWorkspace(
    {
      agentWorkspace: vi.fn().mockResolvedValue({
        ...agentSnapshot(),
        session: {
          ...agentSnapshot().session!,
          sessionId: "session.old",
          timeline: [{ sequence: 1, kind: "assistant", message: "Old latest answer.", createdAt: "2026-06-30T07:00:00Z" }],
        },
      }),
      createSession,
    },
    { forceEmptyThread: true, onSessionStarted },
  );

  await screen.findByTestId("agent-thread-surface");
  expect(screen.getByText("Ask DesktopLab what to change, inspect, or verify in this repository.")).toBeInTheDocument();
  expect(screen.queryByText("Old latest answer.")).not.toBeInTheDocument();

  fireEvent.change(screen.getByRole("textbox", { name: "Prompt" }), { target: { value: "Start a fresh pass" } });
  fireEvent.click(screen.getByRole("button", { name: "Send prompt" }));

  await waitFor(() =>
    expect(createSession).toHaveBeenCalledWith(expect.objectContaining({
      initialPrompt: "Start a fresh pass",
      newChat: true,
    })),
  );
  await waitFor(() => expect(onSessionStarted).toHaveBeenCalledWith(expect.objectContaining({ sessionId: "session.new" })));
});

test("empty workbench actions insert real prompts only when repository context exists", async () => {
  const { unmount } = renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshot(),
      session: null,
    } satisfies AgentWorkspaceSnapshot),
  });

  await screen.findByTestId("agent-thread-surface");
  fireEvent.click(screen.getByRole("button", { name: "Explain this project" }));
  expect(screen.getByRole("textbox", { name: "Prompt" })).toHaveValue("Explain this project and summarize how it is structured.");
  unmount();

  const client = {
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshot(),
      context: null,
      session: null,
    } satisfies AgentWorkspaceSnapshot),
    sessionControl: vi.fn().mockResolvedValue({ accepted: true }),
    createSession: vi.fn(),
  } as unknown as DesktopLabApiClient;

  render(
    <AppProviders apiClient={client}>
      <AgentWorkspaceFeature workspaceId="workspace.desktoplab" workspaceName="desktoplab" onOpenChanges={vi.fn()} onOpenApprovals={vi.fn()} onOpenSetup={vi.fn()} />
    </AppProviders>,
  );

  await screen.findByTestId("agent-thread-surface");
  expect(screen.queryByRole("button", { name: "Explain this project" })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Review changes" })).not.toBeInTheDocument();
});

test("session changes stay out of the thread even when approvals are present", async () => {
  const onOpenChanges = vi.fn();
  renderAgentWorkspace(
    {
      agentWorkspace: vi.fn().mockResolvedValue({
        ...agentSnapshot(),
        session: {
          ...agentSnapshot().session!,
          state: "blocked",
          timeline: [
            ...agentSnapshot().session!.timeline,
            {
              sequence: 4,
              kind: "approval",
              message: "Approval required",
              createdAt: "2026-06-26T08:03:00Z",
            },
          ],
        },
      } satisfies AgentWorkspaceSnapshot),
    },
    { onOpenChanges },
  );

  await screen.findByTestId("agent-thread-surface");

  expect(screen.queryByRole("complementary", { name: "Session changes" })).not.toBeInTheDocument();
  expect(screen.queryByText("Dirty worktree")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Review changes" })).not.toBeInTheDocument();
  expect(onOpenChanges).not.toHaveBeenCalled();
});

test("explains cloud route account mode without exposing backend ids", async () => {
  renderAgentWorkspace({
    agentWorkspace: vi.fn().mockResolvedValue({
      ...agentSnapshot(),
      route: {
        ...agentSnapshot().route!,
        backendKind: "cloud",
        backendDisplayName: "Claude",
        summary: "Uses your signed-in Claude account",
        accountMode: "subscription_account",
        reasons: ["Policy allows this provider for the current prompt"],
      },
    } satisfies AgentWorkspaceSnapshot),
  });

  await screen.findByTestId("agent-thread-surface");

  expect(screen.queryByText("Subscription account")).not.toBeInTheDocument();
  expect(screen.queryByText("Cloud route. Repository context leaves this computer only after account and policy approval.")).not.toBeInTheDocument();
  expect(screen.queryByText("backend.ollama")).not.toBeInTheDocument();
});

function renderAgentWorkspace(
  overrides: Partial<DesktopLabApiClient> = {},
  callbacks: {
    onOpenChanges?: () => void;
    onOpenApprovals?: () => void;
    onOpenSetup?: () => void;
    onSessionStarted?: (session: NonNullable<AgentWorkspaceSnapshot["session"]>) => void;
    forceEmptyThread?: boolean;
    selectedSession?: NonNullable<AgentWorkspaceSnapshot["session"]>;
  } = {},
) {
  const client = {
    agentWorkspace: vi.fn().mockResolvedValue(agentSnapshot()),
    approvalModes: vi.fn().mockResolvedValue(approvalModes()),
    updateSessionApprovalMode: vi.fn().mockResolvedValue(approvalModes()),
    routeOptions: vi.fn().mockResolvedValue(routeOptions()),
    updateRouteSelection: vi.fn().mockResolvedValue(routeOptions()),
    gitOperations: vi.fn().mockResolvedValue(gitOperations()),
    contextAttachments: vi.fn().mockResolvedValue(contextAttachments()),
    sessionControl: vi.fn().mockResolvedValue({ accepted: true }),
    listApprovals: vi.fn().mockResolvedValue({ approvals: [] }),
    createApproval: vi.fn().mockResolvedValue({ approvalId: "approval.1", sessionId: "session.pending", action: "provider.egress", operationId: "provider.openai:route.external.codex:workspace.desktoplab", state: "pending" }),
    resolveApproval: vi.fn().mockResolvedValue({ approvalId: "approval.1", state: "approved" }),
    createSession: vi.fn(),
    continueSession: vi.fn(),
    ...overrides,
  } as unknown as DesktopLabApiClient;

  return render(
    <AppProviders apiClient={client}>
      <AgentWorkspaceFeature
        workspaceId="workspace.desktoplab"
        workspaceName="desktoplab"
        selectedSession={callbacks.selectedSession ?? null}
        forceEmptyThread={callbacks.forceEmptyThread ?? false}
        onOpenChanges={callbacks.onOpenChanges ?? vi.fn()}
        onOpenApprovals={callbacks.onOpenApprovals ?? vi.fn()}
        onOpenSetup={callbacks.onOpenSetup ?? vi.fn()}
        onSessionStarted={callbacks.onSessionStarted}
      />
    </AppProviders>,
  );
}

function writeApproval(): ApprovalSummary {
  return {
    approvalId: "approval.write.1",
    sessionId: "session.1",
    action: "filesystem.write",
    state: "pending",
    risk: "medium",
    title: "Write DESKTOPLAB_AGENT_NOTES.md",
    message: "DesktopLab wants to create or edit DESKTOPLAB_AGENT_NOTES.md.",
    requestedAt: "2026-07-01T14:59:00Z",
    policyReason: "Filesystem writes need approval.",
  };
}

function terminalApproval(): ApprovalSummary {
  return {
    approvalId: "approval.terminal.1",
    sessionId: "session.1",
    action: "terminal.command",
    operationId: "terminal:npm test",
    state: "pending",
    risk: "medium",
    title: "Run npm test",
    message: "DesktopLab wants to run `npm test` in the workspace terminal.",
    requestedAt: "2026-07-01T14:59:00Z",
  };
}

function gitOperations(): GitOperationsSnapshot {
  return {
    workspaceId: "workspace.desktoplab",
    workspaceState: "dirty",
    warnings: ["Dirty worktree"],
    changedFiles: ["README.md"],
    diffPreview: "diff --git a/README.md b/README.md",
    savePoints: [],
    commit: {
      supported: true,
      sessionId: "session.1",
      message: "agent change",
      preview: "Commit requires approval.",
      changeFingerprint: "sha256:test-diff",
      requiresApproval: true,
    },
    push: {
      supported: false,
      remote: "origin",
      branch: "main",
      preview: "Push is unavailable until a commit exists.",
      requiresApproval: true,
      normalizedReason: "no_commit",
    },
    worktrees: [],
  };
}

function approvalModes(
  sessionMode:
    | "require_approval"
    | "approve_for_me"
    | "approve_workspace_writes_for_session"
    | "full_access" = "require_approval",
) {
  return {
    defaultMode: "require_approval" as const,
    sessionMode,
    modes: [
      {
        mode: "require_approval" as const,
        label: "Ask for approval",
        description: "Recommended for small local models and first-time setup.",
      },
      {
        mode: "approve_for_me" as const,
        label: "Approve routine actions",
        description: "DesktopLab can approve routine local steps while provider egress still stops.",
      },
      {
        mode: "approve_workspace_writes_for_session" as const,
        label: "Allow workspace writes",
        description: "Workspace file writes can continue while commands and git actions still stop.",
      },
      {
        mode: "full_access" as const,
        label: "Full local access",
        description: "External providers and protected data still stop for you.",
      },
    ],
  };
}

function routeOptions() {
  return {
    selectedRouteId: "route.local.qwen-coder-7b",
    options: [
      {
        routeId: "route.local.qwen-coder-7b",
        backendId: "backend.ollama",
        backendKind: "local",
        label: "Qwen Coder 7B · Ollama",
        modelId: "model.qwen-coder-7b-q4",
        runtimeId: "runtime.ollama",
        executionBackendId: "backend.ollama",
        modelDisplayName: "Qwen Coder 7B",
        runtimeDisplayName: "Ollama",
        status: "available",
      },
      {
        routeId: "route.local.deepseek-coder-7b-q4",
        backendId: "backend.ollama",
        backendKind: "local",
        label: "DeepSeek Coder 7B · Ollama",
        modelId: "model.deepseek-coder-7b-q4",
        runtimeId: "runtime.ollama",
        executionBackendId: "backend.ollama",
        modelDisplayName: "DeepSeek Coder 7B",
        runtimeDisplayName: "Ollama",
        status: "available",
      },
    ],
  };
}

function routeOptionsWithBlockedAlternatives() {
  return {
    selectedRouteId: "route.local.qwen-coder-7b",
    options: [
      routeOptions().options[0],
      {
        routeId: "route.cloud.openai",
        backendId: "backend.openai",
        backendKind: "cloud" as const,
        label: "OpenAI GPT-4.1 · Cloud",
        modelDisplayName: "GPT-4.1",
        runtimeDisplayName: "OpenAI",
        status: "unavailable" as const,
        disabledReason: "Connect OpenAI before routing work to the cloud.",
      },
      {
        routeId: "route.external.codex",
        backendId: "backend.codex",
        backendKind: "external" as const,
        label: "Codex bridge · External",
        modelDisplayName: "Codex",
        runtimeDisplayName: "Codex",
        status: "unavailable" as const,
        disabledReason: "Codex bridge is not connected yet.",
      },
    ],
  };
}

function routeOptionsWithCodexBridge(selectedRouteId = "route.local.qwen-coder-7b") {
  return {
    selectedRouteId,
    options: [
      routeOptions().options[0],
      {
        routeId: "route.external.codex",
        backendId: "backend.codex",
        backendKind: "external" as const,
        label: "Codex bridge · External",
        modelDisplayName: "Codex",
        runtimeDisplayName: "Codex",
        status: "available" as const,
        egressPolicy: "requires_approval",
        repositoryContextEgress: "approval_required",
      },
    ],
  };
}

function contextAttachments() {
  return {
    workspaceId: "workspace.desktoplab",
    attachments: [
      { path: "README.md", label: "README.md", state: "available" },
      { path: ".env", label: ".env", state: "unavailable", disabledReason: "Protected local file." },
    ],
  };
}

function agentSnapshot(sessionState: NonNullable<AgentWorkspaceSnapshot["session"]>["state"] = "completed"): AgentWorkspaceSnapshot {
  return {
    route: {
      status: "selected",
      backendId: "backend.ollama",
      backendDisplayName: "Local runner",
      backendKind: "local",
      modelDisplayName: "Qwen Coder 7B",
      runtimeDisplayName: "Ollama",
      modelAgentCapability: {
        class: "limited_agent_capable",
        routeLabel: "Limited local agent",
        claim: "Needs live-local certification before full coding-agent routing.",
      },
      summary: "Runs on this machine",
      reasons: ["No provider access required"],
      requiredCapabilities: ["Chat", "Tool use", "Test execution"],
      needsFallbackApproval: false,
    } as AgentWorkspaceSnapshot["route"] & { modelDisplayName: string; runtimeDisplayName: string },
    context: {
      workspaceId: "workspace.desktoplab",
      languages: ["TypeScript", "Rust"],
      frameworks: ["React", "Tauri"],
      testCommands: [{ command: "npm run check", confidence: "confirmed" }],
      protectedSummary: [".env and credential files are excluded"],
      stale: false,
      refreshSupported: true,
    },
    session: {
      sessionId: "session.1",
      workspaceId: "workspace.desktoplab",
      executionBackendId: "backend.ollama",
      owner: "desktoplab",
      state: sessionState,
      plan: "Inspect, edit, test.",
      checkpoints: ["checkpoint.1"],
      summary: "2 files changed",
      controls: { pause: true, resume: false, cancel: true },
      timeline: [
        {
          sequence: 1,
          kind: "planning",
          message: "Read repository",
          createdAt: "2026-06-26T08:00:00Z",
        },
        {
          sequence: 2,
          kind: "tool",
          message: "Terminal command",
          createdAt: "2026-06-26T08:01:00Z",
          evidence: { title: "Command output", body: "npm run check PASS", redacted: true },
        },
        {
          sequence: 3,
          kind: "test",
          message: "Tests passed",
          createdAt: "2026-06-26T08:02:00Z",
          test: { state: "passed", command: "npm run check", output: "PASS" },
        },
      ],
    },
  };
}

function agentSnapshotWithCodexRoute(): AgentWorkspaceSnapshot {
  return {
    ...agentSnapshot(),
    route: {
      status: "selected",
      backendId: "backend.codex",
      backendDisplayName: "Codex bridge",
      backendKind: "external",
      modelDisplayName: "Codex",
      runtimeDisplayName: "Codex",
      accountMode: "subscription_account",
      summary: "Uses your connected Codex app session",
      reasons: ["External route is connected"],
      requiredCapabilities: ["Chat", "Tool use"],
      needsFallbackApproval: false,
    },
  };
}

function agentSnapshotWithApproval(): AgentWorkspaceSnapshot {
  return {
    ...agentSnapshot("blocked"),
    session: {
      ...agentSnapshot("blocked").session!,
      timeline: [
        ...agentSnapshot("blocked").session!.timeline,
        {
          sequence: 4,
          kind: "approval",
          message: "Needs approval · Write DESKTOPLAB_AGENT_NOTES.md",
          createdAt: "2026-07-01T14:59:00Z",
        },
      ],
    },
  };
}

function agentSnapshotWithoutSession(snapshot: AgentWorkspaceSnapshot = agentSnapshot()): AgentWorkspaceSnapshot {
  return { ...snapshot, session: null };
}
