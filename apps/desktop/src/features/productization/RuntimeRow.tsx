import type { RuntimeInstallResponse, RuntimeInventoryItem, RuntimeStopResponse } from "../../api/types";
import { CapabilityList, EvidenceDisclosure, RepairActionRow, StatusRow } from "../../design/OperationalPrimitives";
import { RuntimeInstallStatePanel } from "./RuntimeInstallStatePanel";
import { RuntimeLifecyclePanel } from "./RuntimeLifecyclePanel";
import { setupFailureCopy } from "../setup/setupFailureCopy";

export function RuntimeRow({
  runtime,
  installState,
  stopState,
  onInstall,
  onStop,
  installing,
  stopping,
}: {
  runtime: RuntimeInventoryItem;
  installState?: RuntimeInstallResponse;
  stopState?: RuntimeStopResponse;
  onInstall: () => void;
  onStop: () => void;
  installing: boolean;
  stopping: boolean;
}) {
  const supported = runtime.install.supported;
  const detail = runtime.version
    ? `Version ${runtime.version}`
    : runtime.ownership === "externally_managed"
      ? "External runner"
      : runtime.ownership === "user_owned"
        ? "Already installed on this computer"
        : "Ready for setup";
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <StatusRow label={runtime.displayName} status={runtime.status} detail={detail} />
      <div className="mt-3">
        <CapabilityList capabilities={runtime.capabilities} />
      </div>
      {supported ? (
        <div className="mt-3">
          <RepairActionRow
            label={`Install ${runtime.displayName}`}
            description="DesktopLab can install this local runner on demand."
            disabled={installing}
            onClick={onInstall}
          />
        </div>
      ) : (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">
          {setupFailureCopy(runtime.install.blockedReason) ?? "Install this runner outside DesktopLab."}
        </p>
      )}
      {runtime.lifecycle?.stop.state === "supported" ? (
        <div className="mt-3">
          <RepairActionRow
            label={`Stop ${runtime.displayName}`}
            description={runtime.lifecycle.stop.reason}
            disabled={stopping}
            onClick={onStop}
          />
        </div>
      ) : null}
      <RuntimeLifecyclePanel runtime={runtime} />
      {installState ? <RuntimeInstallStatePanel install={installState} /> : null}
      {stopState ? (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">
          {stopState.state === "completed"
            ? `${runtime.displayName} stopped. DesktopLab preserved the managed files for a later restart.`
            : stopState.remediation ?? `DesktopLab did not stop ${runtime.displayName}.`}
        </p>
      ) : null}
      {runtime.logExcerpt ? (
        <div className="mt-3">
          <EvidenceDisclosure title="Runner diagnostics" body={runtime.logExcerpt} />
        </div>
      ) : null}
    </section>
  );
}
