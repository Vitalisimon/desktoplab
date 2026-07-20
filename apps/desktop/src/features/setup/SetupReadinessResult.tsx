import { AlertCircle, CheckCircle2, FolderOpen, ShieldAlert } from "../../design/icons";
import type { ReadinessResponse } from "../../api/types";
import { setupFailureCopy } from "./setupFailureCopy";

type SetupReadinessResultProps = {
  readiness: ReadinessResponse;
  onOpenRepository: () => void;
  actionLabel?: string;
};

export function SetupReadinessResult({ readiness, onOpenRepository, actionLabel = "Open Repository" }: SetupReadinessResultProps) {
  const state = stateUi(readiness.state);
  const reasons = (readiness.degradedReasons ?? [])
    .map((reason) => setupFailureCopy(reason))
    .filter((reason): reason is string => Boolean(reason));
  const canOpenRepository = readiness.state !== "blocked" && readiness.state !== "starting";

  return (
    <section aria-labelledby="setup-readiness-title" className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start gap-3">
        <state.Icon className={state.iconClassName} size={20} />
        <div className="min-w-0">
          <p className={`text-sm font-semibold ${state.textClassName}`}>{state.label}</p>
          <h2 id="setup-readiness-title" className="mt-1 text-lg font-semibold">
            {state.title}
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">{state.description}</p>
        </div>
      </div>

      {reasons.length > 0 ? (
        <ul className="mt-4 space-y-2">
          {reasons.map((reason) => (
            <li key={reason} className="rounded-desktop bg-elevated px-3 py-2 text-sm text-ink">
              {reason}
            </li>
          ))}
        </ul>
      ) : null}

      <button
        type="button"
        onClick={onOpenRepository}
        disabled={!canOpenRepository}
        className="mt-4 inline-flex h-10 items-center gap-2 rounded-desktop bg-ink px-4 text-sm font-semibold text-canvas disabled:cursor-not-allowed disabled:bg-muted"
      >
        <FolderOpen size={15} />
        {actionLabel}
      </button>
    </section>
  );
}

function stateUi(state: ReadinessResponse["state"]) {
  switch (state) {
    case "ready":
      return {
        label: "Ready",
        title: "DesktopLab is ready for a repository.",
        description: "Local setup is verified. Nothing leaves this computer to continue.",
        Icon: CheckCircle2,
        iconClassName: "text-success",
        textClassName: "text-success",
      };
    case "degraded":
      return {
        label: "Degraded",
        title: "Setup finished with limited capability.",
        description: "You can open a repository, but DesktopLab will keep the limitation visible.",
        Icon: AlertCircle,
        iconClassName: "text-warning",
        textClassName: "text-warning",
      };
    case "blocked":
      return {
        label: "Blocked",
        title: "Setup needs attention before coding.",
        description: "Resolve the listed issue, then run setup again.",
        Icon: ShieldAlert,
        iconClassName: "text-danger",
        textClassName: "text-danger",
      };
    case "starting":
      return {
        label: "Starting",
        title: "DesktopLab is checking readiness.",
        description: "DesktopLab is starting its local services.",
        Icon: AlertCircle,
        iconClassName: "text-muted",
        textClassName: "text-muted",
      };
  }
}
