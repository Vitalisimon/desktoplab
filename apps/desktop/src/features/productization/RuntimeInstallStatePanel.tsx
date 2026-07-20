import type { RuntimeInstallResponse } from "../../api/types";
import { displayRuntimeInstallState, displayRuntimeVerificationState } from "../../domain/displayNames";

export function RuntimeInstallStatePanel({ install }: { install: RuntimeInstallResponse }) {
  const failed = install.state === "failed" || install.verificationState === "failed";
  const blocked = install.state === "blocked" || install.state === "external_guided";
  return (
    <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
      <p className={`text-sm font-semibold ${failed ? "text-danger" : blocked ? "text-warning" : "text-ink"}`}>Runtime install {displayRuntimeInstallState(install.state)}</p>
      <p className="mt-1 text-sm text-muted">Verification {displayRuntimeVerificationState(install.verificationState)}</p>
      {install.remediation ? <p className="mt-1 text-sm text-muted">{install.remediation}</p> : null}
      {install.executionEvidence ? <p className="mt-1 font-mono text-xs text-muted">{install.executionEvidence}</p> : null}
      {failed ? (
        <button type="button" className="mt-2 rounded-desktop border border-line bg-panel px-3 py-2 text-sm font-semibold text-ink">
          Open Diagnostics
        </button>
      ) : null}
    </div>
  );
}
