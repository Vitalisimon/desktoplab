// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { BackendEventFrame } from "../../api/events";
import type { AgentSessionSnapshot } from "../../api/types";
import { ConversationTranscript } from "./AgentConversation";
import { displayEventMessage } from "./conversationDisplay";

test("canonical native tool events use user-facing action labels", () => {
  expect(
    displayEventMessage(
      "tool_decision",
      "state=planned source=agent.iterative canonical=desktoplab.patch_file tool=desktoplab.patch_file call_id=patch-1",
    ),
  ).toBe("Planned · Patch file");
  expect(
    displayEventMessage(
      "tool_decision",
      "state=failed source=agent.iterative canonical=desktoplab.run_tests tool=desktoplab.run_tests call_id=test-1",
    ),
  ).toBe("Failed · Run validation");
});

test("hides stale backend progress after a terminal blocked outcome", () => {
  render(<ConversationTranscript session={session()} eventFrames={frames()} />);

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Blocked");
  expect(screen.queryByLabelText("Agent progress")).not.toBeInTheDocument();
});

test("conversation exposes human agent states for running approval and complete sessions", () => {
  const { rerender } = render(<ConversationTranscript session={{ ...session(), state: "running" }} eventFrames={[]} />);

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Working");

  rerender(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        timeline: [{ sequence: 1, kind: "approval", message: "Approve filesystem write", createdAt: "2026-06-26T08:00:00Z" }],
        pendingApprovals: [{ approvalId: "approval.1", sessionId: "session.1", action: "filesystem.write", operationId: "filesystem.write:README.md", state: "pending", risk: "medium", title: "Write README.md", message: "Approve file write", requestedAt: "2026-06-26T08:00:00Z" }],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Waiting for approval");

  rerender(<ConversationTranscript session={{ ...session(), state: "completed", summary: "Done" }} eventFrames={[]} />);

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Complete");

  rerender(<ConversationTranscript session={{ ...session(), state: "failed" }} eventFrames={[]} />);

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Failed");

  rerender(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        timeline: [{ sequence: 1, kind: "blocked", message: "clarification_required:file_target", createdAt: "2026-06-26T08:00:00Z" }],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Needs input");
});

test("conversation shows user prompt without backend repository context internals", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        plan: "Funzioni?",
        timeline: [
          {
            sequence: 1,
            kind: "planning",
            message: "Funzioni?\n\nRepository context: repository: files=README.md",
            createdAt: "2026-06-26T08:00:00Z",
          },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Funzioni?")).toBeInTheDocument();
  expect(screen.getAllByText("Funzioni?")).toHaveLength(1);
  expect(screen.queryByText(/Repository context:/)).not.toBeInTheDocument();
});

test("conversation maps local inference failure codes to user-readable copy", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        timeline: [{ sequence: 1, kind: "blocked", message: "local_inference_failed", createdAt: "2026-06-26T08:00:00Z" }],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Local inference failed before the agent could continue.")).toBeInTheDocument();
  expect(screen.queryByText("local_inference_failed")).not.toBeInTheDocument();
});

test("conversation maps terminal executor codes to user-readable copy", () => {
  const { rerender } = render(
    <ConversationTranscript
      session={{ ...session(), timeline: [{ sequence: 1, kind: "blocked", message: "approval denied", createdAt: "2026-07-15T08:00:00Z" }] }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Action was not approved.")).toBeInTheDocument();
  expect(screen.queryByText("approval denied")).not.toBeInTheDocument();

  rerender(
    <ConversationTranscript
      session={{ ...session(), timeline: [{ sequence: 1, kind: "blocked", message: "path_escape", createdAt: "2026-07-15T08:00:00Z" }] }}
      eventFrames={[]}
    />,
  );
  expect(screen.getByText("DesktopLab blocked access outside this repository.")).toBeInTheDocument();
});

test("conversation maps provider recovery codes to user-readable progress", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "running",
        timeline: [
          {
            sequence: 1,
            kind: "tool_decision",
            message: "provider_output_recovery:initial_malformed_retry",
            createdAt: "2026-06-26T08:00:00Z",
          },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent progress")).toHaveTextContent("Refining the next action");
  expect(screen.queryByText(/provider_output_recovery/)).not.toBeInTheDocument();
});

test("conversation keeps terminal failure outcomes visible beside the backend transcript", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        summary: null,
        transcript: [{ sequence: 1, role: "user", content: "Inspect repository" }],
        timeline: [
          { sequence: 1, kind: "user", message: "Inspect repository", createdAt: "2026-06-26T08:00:00Z" },
          { sequence: 2, kind: "failed", message: "local_inference_failed", createdAt: "2026-06-26T08:00:01Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Inspect repository")).toBeInTheDocument();
  expect(screen.getByText("Local inference failed before the agent could continue.")).toBeInTheDocument();
});

test("conversation renders stable failure copy instead of backend stop jargon", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        transcript: [{ sequence: 1, role: "user", content: "Create notes.md" }],
        timeline: [{ sequence: 2, kind: "failed", message: "malformed structured file action", createdAt: "2026-06-26T08:00:01Z" }],
        failureClassification: {
          schemaVersion: 1,
          primary: "tool_misuse",
          findings: [{ code: "tool_misuse", message: "The selected tool did not match the requested operation." }],
          originalStopReason: "malformed structured file action",
          userMessage: "The selected tool did not match the requested operation.",
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("The selected tool did not match the requested operation.")).toBeInTheDocument();
  expect(screen.queryByText("malformed structured file action")).not.toBeInTheDocument();
});

test("conversation renders assistant prose cleanly and strips control artifacts", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        timeline: [
          {
            sequence: 1,
            kind: "assistant",
            message: "Questa miniapp sembra gestire contatti \u001b[8D\u001b[Kcontatti da SQLite.",
            createdAt: "2026-06-26T08:00:00Z",
          },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Questa miniapp sembra gestire contatti da SQLite.")).toHaveClass("text-ink");
  expect(screen.queryByText(/\u001b/)).not.toBeInTheDocument();
  expect(screen.queryByText("Command output")).not.toBeInTheDocument();
});

test("conversation hides internal tool and completion codes from completed threads", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        summary: "agent loop completed",
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "filesystem.read:README.md", createdAt: "2026-06-26T08:00:00Z" },
          { sequence: 2, kind: "completed", message: "agent loop completed", createdAt: "2026-06-26T08:01:00Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Response complete")).toHaveClass("text-muted");
  expect(screen.queryByText("Read README.md")).not.toBeInTheDocument();
  expect(screen.queryByText("filesystem.read:README.md")).not.toBeInTheDocument();
  expect(screen.queryByText("agent loop completed")).not.toBeInTheDocument();
});

test("conversation removes executor metadata from visible answers and preserves line breaks", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        transcript: [
          { sequence: 1, role: "user", content: "Summarize the change" },
          { sequence: 2, role: "assistant", content: "Updated the file.\n\n- kept the heading\n- fixed the copy\nredacted=true redaction_source=policy" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  const answer = screen.getByText(/Updated the file/);
  expect(answer).toHaveClass("whitespace-pre-wrap");
  expect(answer).toHaveTextContent("kept the heading");
  expect(screen.queryByText(/redaction_source/)).not.toBeInTheDocument();
});

test("conversation keeps raw diff metadata and clarification routing out of visible output", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        transcript: [{ sequence: 1, role: "assistant", content: "Change reviewed. Git diff: redacted=true redaction_source=git.diff diff --git a/a.md b/a.md clarification_required:Explain it" }],
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=observed source=agent.clarify tool=clarify:Explain approval_mode=require_approval", createdAt: "2026-07-10T10:00:00Z" },
          { sequence: 2, kind: "blocked", message: "clarification_required:Explain it", createdAt: "2026-07-10T10:00:01Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Change reviewed.")).toBeInTheDocument();
  expect(screen.getByLabelText("Agent progress")).toHaveTextContent("Waiting for clarification");
  expect(screen.getByLabelText("Agent progress")).not.toHaveTextContent("state=observed");
  expect(screen.queryByText(/redaction_source/)).not.toBeInTheDocument();
});

test("conversation suppresses standalone diff transport and explains clarification events", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        timeline: [
          { sequence: 1, kind: "assistant", message: "Git diff: redacted=true redaction_source=git.diff diff --git a/a.md b/a.md", createdAt: "2026-07-10T10:00:00Z" },
          { sequence: 2, kind: "blocked", message: "clarification_required:Explain the change", createdAt: "2026-07-10T10:00:01Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.queryByText(/redaction_source/)).not.toBeInTheDocument();
  expect(screen.queryByText(/clarification_required/)).not.toBeInTheDocument();
  expect(screen.getByText("Clarification needed: Explain the change")).toBeInTheDocument();
});

test("completed conversation collapses streamed progress events", () => {
  render(<ConversationTranscript session={{ ...session(), state: "completed" }} eventFrames={frames()} />);

  expect(screen.getByLabelText("Agent status")).toHaveTextContent("Complete");
  expect(screen.queryByLabelText("Agent progress")).not.toBeInTheDocument();
  expect(screen.queryByText("Prompt accepted")).not.toBeInTheDocument();
  expect(screen.queryByText("Repository context read")).not.toBeInTheDocument();
});

test("conversation collapses progress after assistant output reaches a terminal block", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        timeline: [
          { sequence: 1, kind: "planning", message: "Leggi il file", createdAt: "2026-07-01T08:00:00Z" },
          { sequence: 2, kind: "tool_decision", message: "state=planned source=filesystem.read tool=filesystem.read:DESKTOPLAB_AGENT_NOTES.md", createdAt: "2026-07-01T08:00:01Z" },
          { sequence: 3, kind: "tool_decision", message: "state=executed source=filesystem.read tool=filesystem.read:DESKTOPLAB_AGENT_NOTES.md", createdAt: "2026-07-01T08:00:02Z" },
          { sequence: 4, kind: "assistant", message: "Read DESKTOPLAB_AGENT_NOTES.md:\ncontenuto reale", createdAt: "2026-07-01T08:00:03Z" },
        ],
      }}
      eventFrames={frames()}
    />,
  );

  expect(screen.getByText(/contenuto reale/)).toBeInTheDocument();
  expect(screen.queryByLabelText("Agent progress")).not.toBeInTheDocument();
  expect(screen.queryByText("Planned · Read DESKTOPLAB_AGENT_NOTES.md")).not.toBeInTheDocument();
  expect(screen.queryByText("Read DESKTOPLAB_AGENT_NOTES.md")).not.toBeInTheDocument();
});

test("classified failures do not revive an earlier approval wait", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        failureClassification: {
          schemaVersion: 1,
          primary: "validation_failed",
          findings: [{ code: "validation_failed", message: "The latest validation command failed." }],
          originalStopReason: "provider_declared_failed",
          userMessage: "The latest validation command failed. Review the output, repair the issue, and run it again.",
        },
        transcript: [{ sequence: 1, role: "user", content: "Run the tests" }],
        timeline: [
          { sequence: 1, kind: "blocked", message: "waiting for approval", createdAt: "2026-07-01T08:00:01Z" },
          { sequence: 2, kind: "failed", message: "provider_declared_failed", createdAt: "2026-07-01T08:00:02Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText(/latest validation command failed/i)).toBeInTheDocument();
  expect(screen.queryByText("Waiting for approval.")).not.toBeInTheDocument();
});

test("a stale session summary does not duplicate an earlier assistant turn", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        summary: "Created VISUAL_QA.md.",
        transcript: [
          { sequence: 1, role: "user", content: "Create VISUAL_QA.md" },
          { sequence: 2, role: "assistant", content: "Created VISUAL_QA.md." },
          { sequence: 3, role: "user", content: "Run a failing test" },
        ],
        timeline: [{ sequence: 4, kind: "failed", message: "provider_declared_failed", createdAt: "2026-07-01T08:00:02Z" }],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getAllByText("Created VISUAL_QA.md.")).toHaveLength(1);
});

test("conversation collapses planned approval and executed tool events into technical evidence", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "blocked",
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=planned source=filesystem.preview tool=filesystem.write:README.md", createdAt: "2026-06-26T08:00:00Z" },
          { sequence: 2, kind: "tool_decision", message: "state=approval_required source=filesystem.write tool=filesystem.write:README.md", createdAt: "2026-06-26T08:00:00Z" },
          { sequence: 3, kind: "tool_decision", message: "state=executed source=filesystem.write tool=filesystem.write:README.md", createdAt: "2026-06-26T08:00:00Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.queryByLabelText("Agent progress")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Technical evidence timeline")).toBeInTheDocument();
  expect(screen.getAllByText("Result recorded with sensitive details removed")).toHaveLength(3);
  expect(screen.queryByText(/state=planned/)).not.toBeInTheDocument();
  expect(screen.queryByText("File write README.md")).not.toBeInTheDocument();
});

test("conversation distinguishes a localized patch from a full file write", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "running",
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=executed source=filesystem.patch tool=filesystem.patch:README.md approval_mode=require_approval", createdAt: "2026-07-15T08:00:00Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent progress")).toHaveTextContent("Patched README.md");
});

test("technical evidence omits internal recovery rows without structured tool fields", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        details: {
          plan: "Read README.md",
          toolCalls: [
            { state: "", source: "", tool: "", approvalMode: "" },
            { state: "observed", source: "filesystem.read", tool: "filesystem.read:README.md", approvalMode: "require_approval" },
          ],
          approvals: [],
          observations: [],
          diffs: [],
          validations: [],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.queryByText("state= source= tool=")).not.toBeInTheDocument();
  expect(screen.getByText("Read README.md")).toBeInTheDocument();
});

test("technical evidence translates every canonical read and validation action", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        details: {
          plan: "Inspect and validate",
          toolCalls: [
            { state: "executed", source: "filesystem.list", tool: "filesystem.list:src", approvalMode: "require_approval" },
            { state: "executed", source: "workspace.search", tool: "search.text:src:needle", approvalMode: "require_approval" },
            { state: "executed", source: "test.runner", tool: "test.run:npm test", approvalMode: "require_approval" },
            { state: "executed", source: "git.read", tool: "git.status", approvalMode: "require_approval" },
            { state: "executed", source: "git.read", tool: "git.diff:README.md", approvalMode: "require_approval" },
            { state: "executed", source: "git.checkpoint", tool: "checkpoint.create:before changes", approvalMode: "require_approval" },
          ],
          approvals: [],
          observations: [],
          diffs: [],
          validations: [],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Listed files")).toBeInTheDocument();
  expect(screen.getByText("Searched workspace")).toBeInTheDocument();
  expect(screen.getByText("Validation completed")).toBeInTheDocument();
  expect(screen.getByText("Inspected Git status")).toBeInTheDocument();
  expect(screen.getByText("Inspected Git diff")).toBeInTheDocument();
  expect(screen.getByText("Safety checkpoint ready")).toBeInTheDocument();
  expect(screen.queryByText(/state=|source=|test\.run:|checkpoint\.create:/)).not.toBeInTheDocument();
});

test("conversation keeps user prompts and assistant responses in timeline order", () => {
  const { container } = render(
    <ConversationTranscript
      session={{
        ...session(),
        plan: "Second prompt",
        state: "completed",
        timeline: [
          { sequence: 1, kind: "planning", message: "First prompt", createdAt: "2026-07-01T08:00:00Z" },
          { sequence: 2, kind: "assistant", message: "First response", createdAt: "2026-07-01T08:01:00Z" },
          { sequence: 3, kind: "planning", message: "Second prompt", createdAt: "2026-07-01T08:02:00Z" },
          { sequence: 4, kind: "assistant", message: "Second response", createdAt: "2026-07-01T08:03:00Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  const text = container.textContent ?? "";
  expect(text.indexOf("First prompt")).toBeLessThan(text.indexOf("First response"));
  expect(text.indexOf("First response")).toBeLessThan(text.indexOf("Second prompt"));
  expect(text.indexOf("Second prompt")).toBeLessThan(text.indexOf("Second response"));
  expect(screen.getByText("Second prompt")).toHaveClass("bg-elevated");
  expect(screen.getAllByText("Second prompt")).toHaveLength(1);
});

test("conversation prefers backend transcript over raw technical timeline", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        transcript: [
          { sequence: 1, role: "user", content: "Crea note.md" },
          { sequence: 2, role: "assistant", content: "Creo note.md." },
        ],
        details: {
          plan: "Crea note.md",
          toolCalls: [{ state: "approval_required", source: "filesystem.write", tool: "filesystem.write:note.md", approvalMode: "require_approval" }],
          approvals: [{ state: "approval_required", source: "filesystem.write", tool: "filesystem.write:note.md", approvalMode: "require_approval" }],
          observations: [],
          diffs: [],
          validations: [],
        },
        timeline: [
          { sequence: 1, kind: "assistant", message: "{\"desktoplabAction\":{\"kind\":\"create_file\"}}", createdAt: "2026-07-01T08:00:00Z" },
          { sequence: 2, kind: "tool_decision", message: "filesystem.write:note.md", createdAt: "2026-07-01T08:00:01Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Creo note.md.")).toBeInTheDocument();
  expect(screen.getByText("Crea note.md")).toHaveClass("bg-elevated");
  expect(screen.getByLabelText("Technical evidence timeline")).toBeInTheDocument();
  expect(screen.queryByText(/desktoplabAction/)).not.toBeInTheDocument();
  expect(screen.queryByText("filesystem.write:note.md")).not.toBeInTheDocument();
});

test("conversation hides raw provider tool-call transcript turns", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        transcript: [
          { sequence: 1, role: "user", content: "Leggi README.md" },
          { sequence: 2, role: "assistant", content: "{\"name\":\"read_file\",\"arguments\":{\"path\":\"README.md\"}}" },
        ],
        details: {
          plan: "Leggi README.md",
          toolCalls: [{ state: "executed", source: "filesystem.read", tool: "filesystem.read:README.md", approvalMode: "require_approval" }],
          approvals: [],
          observations: [{ kind: "observation", message: "Read README.md" }],
          diffs: [],
          validations: [],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Leggi README.md")).toBeInTheDocument();
  expect(screen.queryByText(/read_file/)).not.toBeInTheDocument();
  expect(screen.queryByText(/arguments/)).not.toBeInTheDocument();
});

test("conversation hides concatenated raw provider tool-call transcript turns", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        transcript: [
          { sequence: 1, role: "user", content: "Leggi README.md" },
          {
            sequence: 2,
            role: "assistant",
            content: "{\"name\":\"read_file\",\"arguments\":{\"path\":\"README.md\"}} {\"name\":\"read_file\",\"arguments\":{\"path\":\"src/lib.rs\"}}",
          },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Leggi README.md")).toBeInTheDocument();
  expect(screen.queryByText(/read_file/)).not.toBeInTheDocument();
  expect(screen.queryByText(/arguments/)).not.toBeInTheDocument();
});

test("conversation renders agent terminal output as collapsed expandable evidence", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        transcript: [
          { sequence: 1, role: "user", content: "Esegui i test" },
          { sequence: 2, role: "assistant", content: "Eseguo la validazione." },
        ],
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=executed source=terminal.command tool=terminal:npm test approval_mode=require_approval", createdAt: "2026-07-01T08:00:01Z" },
          { sequence: 2, kind: "tool", message: "status=exited:0\nstdout:\nPASS\nstderr:\n", createdAt: "2026-07-01T08:00:02Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent terminal executions")).toBeInTheDocument();
  expect(screen.getByText("Command completed · npm test")).toBeInTheDocument();
  expect(screen.getByText(/PASS/)).toBeInTheDocument();
});

test("test evidence does not mistake redaction status for process failure", () => {
  render(
    <ConversationTranscript
      session={terminalSession("Test command `npm test` finished with status Exited(0) in 149ms. redaction_status=redacted stdout_truncated=false stderr_truncated=false\nstdout:\nPASS\nstderr:\n")}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Command completed · npm test")).toBeInTheDocument();
  expect(screen.queryByText("Command failed · npm test")).not.toBeInTheDocument();
});

test("conversation opens failed agent terminal output for diagnosis", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=executed source=terminal.command tool=terminal:npm test approval_mode=require_approval", createdAt: "2026-07-01T08:00:01Z" },
          { sequence: 2, kind: "tool", message: "status=exited:1\nstdout:\n\nstderr:\nexpected 42", createdAt: "2026-07-01T08:00:02Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  const details = screen.getByText("Command failed · npm test").closest("details");
  expect(details).toHaveAttribute("open");
  expect(screen.getByText(/expected 42/)).toBeInTheDocument();
});

test("agent terminal evidence keeps long output scrollable with theme-safe classes", () => {
  const longOutput = Array.from({ length: 80 }, (_, index) => `line ${index}`).join("\n");
  const { container, rerender } = render(
    <div>
      <ConversationTranscript session={terminalSession(`status=exited:0\nstdout:\n${longOutput}\nstderr:\n`)} eventFrames={[]} />
    </div>,
  );

  let details = container.querySelector('[aria-label="Agent terminal executions"] details');
  let output = container.querySelector("pre");
  expect(details).toHaveClass("bg-panel");
  expect(details).toHaveClass("border-line");
  expect(output).toHaveClass("max-h-72");
  expect(output).toHaveClass("overflow-auto");

  rerender(
    <div className="dark">
      <ConversationTranscript session={terminalSession(`status=exited:1\nstdout:\n\nstderr:\n${longOutput}`)} eventFrames={[]} />
    </div>,
  );

  details = container.querySelector('[aria-label="Agent terminal executions"] details');
  output = container.querySelector("pre");
  expect(details).toHaveClass("bg-panel");
  expect(details).toHaveAttribute("open");
  expect(output).toHaveClass("text-ink");
});

test("conversation renders structured diff and failed validation evidence", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "failed",
        details: {
          plan: "Patch and validate",
          toolCalls: [],
          approvals: [],
          observations: [],
          diffs: [{ message: "Git diff:\ndiff --git a/src/lib.rs b/src/lib.rs\n-41\n+42" }],
          validations: [{ message: "Test command `npm test` failed exit=1\nexpected 42" }],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent diff and validation evidence")).toBeInTheDocument();
  expect(screen.getByText("Changed src/lib.rs")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Copy diff" })).toBeInTheDocument();
  const failed = screen.getByText("Validation failed").closest("details");
  expect(failed).toHaveAttribute("open");
});

test("validation evidence recognizes the executor Exited status format", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        details: {
          plan: "Validate",
          toolCalls: [],
          approvals: [],
          observations: [],
          diffs: [],
          validations: [{ message: "Test command `npm test` finished with status Exited(1)" }],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Validation failed").closest("details")).toHaveAttribute("open");
});

test("a passing rerun collapses earlier failed validation evidence", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        details: {
          plan: "Repair and revalidate",
          toolCalls: [],
          approvals: [],
          observations: [],
          diffs: [],
          validations: [
            { message: "Test command `npm test` finished with status Exited(1)" },
            { message: "Test command `npm test` finished with status Exited(0)" },
          ],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByLabelText("Agent diff and validation evidence")).toHaveAttribute("data-evidence-state", "validation-passed");
  expect(screen.getByText("Validation failed").closest("details")).not.toHaveAttribute("open");
  expect(screen.getByText("Validation passed").closest("details")).not.toHaveAttribute("open");
});

test("flattened diffs keep transport details out of the collapsed title", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        details: {
          plan: "Review change",
          toolCalls: [],
          approvals: [],
          observations: [],
          diffs: [{ message: "Git diff: redacted=true redaction_source=git.diff diff --git a/agent-proof.md b/agent-proof.md index abc..def 100644 --- a/agent-proof.md +++ b/agent-proof.md" }],
          validations: [],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Changed agent-proof.md")).toBeInTheDocument();
  expect(screen.queryByText(/Changed agent-proof\.md index/)).not.toBeInTheDocument();
});

test("repeated diff snapshots count each changed file once", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        details: {
          plan: "Repair and validate",
          toolCalls: [],
          approvals: [],
          observations: [],
          diffs: [{
            message: [
              "Git diff:",
              "diff --git a/calculator.js b/calculator.js",
              "-return left - right;",
              "diff --git a/calculator.js b/calculator.js",
              "+return left + right;",
            ].join("\n"),
          }],
          validations: [],
        },
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Changed calculator.js")).toBeInTheDocument();
  expect(screen.queryByText("2 changed files")).not.toBeInTheDocument();
});

test("terminal evidence pairs each result with its command and execution metadata", () => {
  render(
    <ConversationTranscript
      session={{
        ...session(),
        state: "completed",
        timeline: [
          { sequence: 1, kind: "tool_decision", message: "state=executed source=test.runner tool=test.run:npm test approval_mode=require_approval", createdAt: "2026-07-01T08:00:01Z" },
          { sequence: 2, kind: "tool", message: "status=exited:0 duration_ms=125 cwd=.\nstdout:\nPASS\nstderr:\n", createdAt: "2026-07-01T08:00:02Z" },
          { sequence: 3, kind: "tool_decision", message: "state=executed source=test.runner tool=test.run:cargo test approval_mode=require_approval", createdAt: "2026-07-01T08:00:03Z" },
          { sequence: 4, kind: "tool", message: "status=exited:0 duration_ms=250 cwd=crates/core\nstdout:\nok\nstderr:\n", createdAt: "2026-07-01T08:00:04Z" },
        ],
      }}
      eventFrames={[]}
    />,
  );

  expect(screen.getByText("Command completed · npm test")).toBeInTheDocument();
  expect(screen.getByText("Command completed · cargo test")).toBeInTheDocument();
  expect(screen.getByText("125 ms")).toBeInTheDocument();
  expect(screen.getByText("250 ms")).toBeInTheDocument();
  expect(screen.getAllByRole("button", { name: "Copy command output" })).toHaveLength(2);
});

function session(): AgentSessionSnapshot {
  return {
    sessionId: "session.1",
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.ollama",
    owner: "desktoplab",
    state: "blocked",
    plan: "Inspect repository",
    checkpoints: [],
    summary: null,
    timeline: [{ sequence: 1, kind: "blocked", message: "local_inference_not_configured", createdAt: "2026-06-26T08:00:00Z" }],
  };
}

function terminalSession(message: string): AgentSessionSnapshot {
  return {
    ...session(),
    state: message.includes("status=exited:1") ? "failed" : "completed",
    timeline: [
      { sequence: 1, kind: "tool_decision", message: "state=executed source=terminal.command tool=terminal:npm test approval_mode=require_approval", createdAt: "2026-07-01T08:00:01Z" },
      { sequence: 2, kind: "tool", message, createdAt: "2026-07-01T08:00:02Z" },
    ],
  };
}

function frames(): BackendEventFrame[] {
  return [
    frame(1, "agent.prompt.accepted", "Prompt accepted"),
    frame(2, "agent.context.read", "Repository context read"),
    frame(3, "agent.step.blocked", "Action blocked"),
  ];
}

function frame(sequence: number, kind: string, message: string): BackendEventFrame {
  return {
    sequence,
    scope: "session",
    payload: JSON.stringify({ kind, eventId: `session.1.${sequence}`, sessionId: "session.1", message }),
  };
}
