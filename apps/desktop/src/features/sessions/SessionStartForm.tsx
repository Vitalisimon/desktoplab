import { FormEvent, useState } from "react";
import { Play } from "../../design/icons";
import type { SessionCreateRequest } from "../../api/types";
import { displayExecutionBackendName } from "../../domain/displayNames";

type SessionStartFormProps = {
  workspaceId: string;
  backends: string[];
  isCreating?: boolean;
  onCreate: (request: SessionCreateRequest) => void;
};

export function SessionStartForm({ workspaceId, backends, isCreating = false, onCreate }: SessionStartFormProps) {
  const [executionBackendId, setExecutionBackendId] = useState(backends[0] ?? "");
  const [prompt, setPrompt] = useState("");
  const normalizedPrompt = prompt.trim();
  const canSubmit = workspaceId.trim().length > 0 && executionBackendId.length > 0 && normalizedPrompt.length > 0 && !isCreating;

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) return;
    onCreate({
      workspaceId,
      executionBackendId,
      initialPrompt: normalizedPrompt,
    });
  }

  return (
    <form className="rounded-desktop border border-line bg-panel p-4 shadow-sm" onSubmit={submit}>
      <div className="grid gap-4 md:grid-cols-[220px_1fr_auto] md:items-end">
        <div>
          <label className="block text-sm font-semibold" htmlFor="session-backend">
            Agent runner
          </label>
          <select
            id="session-backend"
            value={executionBackendId}
            onChange={(event) => setExecutionBackendId(event.target.value)}
            className="mt-2 h-11 w-full rounded-desktop border border-line bg-panel px-3 text-sm text-ink outline-none ring-accent/20 focus:ring-4"
          >
            {backends.map((backend) => (
              <option key={backend} value={backend}>
                {displayExecutionBackendName(backend)}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-sm font-semibold" htmlFor="session-prompt">
            Prompt
          </label>
          <input
            id="session-prompt"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            className="mt-2 h-11 w-full rounded-desktop border border-line bg-panel px-3 text-sm text-ink outline-none ring-accent/20 focus:ring-4"
            placeholder="Describe the development task"
          />
        </div>

        <button
          type="submit"
          disabled={!canSubmit}
          className="inline-flex h-11 items-center justify-center gap-2 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas disabled:cursor-not-allowed disabled:bg-muted"
        >
          <Play size={15} />
          Start session
        </button>
      </div>
    </form>
  );
}
