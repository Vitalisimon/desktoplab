import { FormEvent, useState } from "react";
import { FolderOpen } from "../../design/icons";
import { DesktopLabApiError } from "../../api/types";
import type { WorkspaceSnapshot } from "../../api/types";
import { chooseRepositoryFolder } from "./repositoryFolderPicker";
import { useWorkspaceOpen } from "./useWorkspaceOpen";

type WorkspaceOpenViewProps = {
  onOpened: (workspace: WorkspaceSnapshot) => void;
};

export function WorkspaceOpenView({ onOpened }: WorkspaceOpenViewProps) {
  const [path, setPath] = useState("");
  const [openError, setOpenError] = useState<string | null>(null);
  const [canInitializeGit, setCanInitializeGit] = useState(false);
  const workspaceOpen = useWorkspaceOpen();
  const normalizedPath = path.trim();

  async function openWorkspace(repositoryPath: string, initializeGit = false) {
    setOpenError(null);
    setCanInitializeGit(false);
    try {
      const request = initializeGit ? { path: repositoryPath, initializeGit: true } : { path: repositoryPath };
      const workspace = await workspaceOpen.open.mutateAsync(request);
      onOpened(workspace);
    } catch (error) {
      if (isGitRequiredError(error)) setCanInitializeGit(true);
      setOpenError(workspaceOpenErrorCopy(error));
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (normalizedPath.length > 0) {
      await openWorkspace(normalizedPath);
      return;
    }
    const selectedPath = await chooseRepositoryFolder();
    if (!selectedPath) return;
    setPath(selectedPath);
    await openWorkspace(selectedPath);
  }

  return (
    <section aria-labelledby="workspace-open-title" className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <div className="flex items-start gap-3">
        <div className="grid h-9 w-9 place-items-center rounded-desktop bg-elevated text-muted">
          <FolderOpen size={18} />
        </div>
        <div>
          <h1 id="workspace-open-title" className="text-2xl font-semibold">
            Open a project folder
          </h1>
          <p className="mt-2 text-sm leading-6 text-muted">
            Choose an existing folder. If it is not a Git repository yet, DesktopLab can initialize it after you confirm.
          </p>
        </div>
      </div>

      <form className="mt-5 space-y-4" onSubmit={submit}>
        <label className="block text-sm font-semibold" htmlFor="workspace-path">
          Repository path
        </label>
        <input
          id="workspace-path"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          className="h-11 w-full rounded-desktop border border-line bg-panel px-3 text-sm text-ink outline-none ring-accent/20 focus:ring-4"
          placeholder={repositoryPathPlaceholder()}
        />

        {openError ? (
          <div className="rounded-desktop bg-warning/10 px-3 py-3 text-sm text-ink">
            <p>{openError}</p>
            {canInitializeGit ? (
              <button
                type="button"
                disabled={workspaceOpen.isOpening}
                onClick={() => void openWorkspace(normalizedPath, true)}
                className="mt-3 inline-flex h-9 items-center rounded-desktop bg-ink px-3 text-sm font-semibold text-canvas disabled:cursor-not-allowed disabled:bg-muted"
              >
                Initialize Git and open
              </button>
            ) : null}
          </div>
        ) : null}

        <button
          type="submit"
          disabled={workspaceOpen.isOpening}
          className="inline-flex h-10 items-center gap-2 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas disabled:cursor-not-allowed disabled:bg-muted"
        >
          <FolderOpen size={15} />
          Open Repository
        </button>
      </form>
    </section>
  );
}

export function repositoryPathPlaceholder(platform = typeof navigator === "undefined" ? "" : navigator.platform): string {
  const normalized = platform.toLowerCase();
  if (normalized.includes("win")) return String.raw`C:\Users\name\project`;
  if (normalized.includes("mac")) return "/Users/name/project";
  return "/home/name/project";
}

function workspaceOpenErrorCopy(error: unknown): string {
  const message = error instanceof DesktopLabApiError || error instanceof Error ? error.message : "";
  if (/not a git repository|\.git|git repository/i.test(message)) {
    return "This folder is not a Git repository yet. DesktopLab can initialize Git here before opening it.";
  }
  return message || "Project folder could not be opened.";
}

function isGitRequiredError(error: unknown): boolean {
  const message = error instanceof DesktopLabApiError || error instanceof Error ? error.message : "";
  return /not a git repository|\.git|git repository/i.test(message);
}
