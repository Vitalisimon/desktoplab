import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ShieldCheck } from "../../design/icons";
import { useApiClient } from "../../api/ApiProvider";
import type { ApprovalMode } from "../../api/types";
import { displayApprovalMode } from "../../domain/displayNames";

export function SafetyApprovalSettings() {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const modes = useQuery({ queryKey: ["settings", "approval-modes"], queryFn: () => api.approvalModes() });
  const updateDefault = useMutation({
    mutationFn: (mode: ApprovalMode) => api.updateDefaultApprovalMode({ mode }),
    onSuccess: (data) => queryClient.setQueryData(["settings", "approval-modes"], data),
  });

  const selected = modes.data?.defaultMode ?? null;
  const selectedLabel = selected ? displayApprovalMode(selected) : null;

  return (
    <section aria-labelledby="safety-approvals-title" className="py-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold" id="safety-approvals-title">
            Safety & Approvals
          </h2>
          <p className="mt-1 max-w-2xl text-sm leading-6 text-muted">
            Choose the default approval behavior for new work sessions. You can still change the active session from the composer.
          </p>
        </div>
        <span className="inline-flex items-center gap-2 rounded-full border border-line bg-elevated px-3 py-1 text-xs font-semibold text-muted">
          <ShieldCheck aria-hidden="true" className="size-3.5" />
          Local guardrails stay on
        </span>
      </div>

      {modes.isLoading ? <p className="mt-4 text-sm text-muted">Reading safety defaults...</p> : null}
      {modes.isError || !modes.data ? <p className="mt-4 text-sm text-danger">Safety defaults are unavailable right now.</p> : null}

      {modes.data ? (
        <>
          <div className="mt-4">
            <p className="text-sm font-medium">Default for new sessions</p>
            <div aria-label="Default approval mode" className="mt-3 grid gap-2 xl:grid-cols-2" role="radiogroup">
              {modes.data.modes.map((mode) => (
                <label
                  className="flex cursor-pointer items-start gap-3 rounded-desktop border border-line bg-elevated px-3 py-3 text-sm transition hover:border-focus"
                  data-selected={mode.mode === selected ? "true" : "false"}
                  key={mode.mode}
                >
                  <input
                    aria-label={displayApprovalMode(mode.mode)}
                    checked={mode.mode === selected}
                    className="mt-1 size-4 accent-current focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
                    disabled={updateDefault.isPending}
                    name="default-approval-mode"
                    onChange={() => updateDefault.mutate(mode.mode)}
                    type="radio"
                  />
                  <span>
                    <span className="block font-medium">{displayApprovalMode(mode.mode)}</span>
                    <span className="block text-xs leading-5 text-muted">{plainDescription(mode.mode)}</span>
                  </span>
                </label>
              ))}
            </div>
          </div>

          <p className="mt-4 text-sm text-muted">
            {selectedLabel ? `${selectedLabel} will be used for new sessions.` : "New sessions will use your saved default."}
          </p>
        </>
      ) : null}
    </section>
  );
}

function plainDescription(mode: ApprovalMode) {
  switch (mode) {
    case "approve_for_me":
      return "DesktopLab can continue through routine local steps while provider egress, pushes and protected data still stop.";
    case "approve_workspace_writes_for_session":
      return "Workspace file writes can continue in this session while commands and git actions still stop.";
    case "full_access":
      return "Commits, pushes, external providers and protected data still stop for you.";
    case "require_approval":
      return "Recommended for small local models and careful first runs.";
  }
}
