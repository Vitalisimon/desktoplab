// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { SessionStartForm } from "./SessionStartForm";

test("keeps session creation disabled until prompt and backend are present", async () => {
  const onCreate = vi.fn();
  render(<SessionStartForm workspaceId="workspace.desktoplab" backends={["backend.ollama"]} onCreate={onCreate} />);

  expect(screen.getByRole("button", { name: /start session/i })).toBeDisabled();

  fireEvent.change(screen.getByLabelText("Prompt"), { target: { value: "Inspect the repository" } });

  expect(screen.getByRole("button", { name: /start session/i })).toBeEnabled();
});

test("submits workspace backend and prompt", async () => {
  const onCreate = vi.fn();
  render(<SessionStartForm workspaceId="workspace.desktoplab" backends={["backend.ollama", "backend.codex"]} onCreate={onCreate} />);

  fireEvent.change(screen.getByLabelText("Agent runner"), { target: { value: "backend.codex" } });
  fireEvent.change(screen.getByLabelText("Prompt"), { target: { value: "Create a plan for the repository" } });
  fireEvent.click(screen.getByRole("button", { name: /start session/i }));

  expect(onCreate).toHaveBeenCalledWith({
    workspaceId: "workspace.desktoplab",
    executionBackendId: "backend.codex",
    initialPrompt: "Create a plan for the repository",
  });
});
