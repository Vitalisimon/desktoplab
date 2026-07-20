import type { RuntimeInstallResponse, RuntimeInventoryItem } from "../../api/types";
import { CapabilityList, EvidenceDisclosure, RepairActionRow, StatusRow } from "../../design/OperationalPrimitives";
import { RuntimeInstallStatePanel } from "./RuntimeInstallStatePanel";
import { RuntimeLifecyclePanel } from "./RuntimeLifecyclePanel";
import { setupFailureCopy } from "../setup/setupFailureCopy";

export function RuntimeRow({
  runtime,
  installState,
  onInstall,
  installing,
}: {
  runtime: RuntimeInventoryItem;
  installState?: RuntimeInstallResponse;
  onInstall: () => void;
  installing: boolean;
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
      <RuntimeLifecyclePanel runtime={runtime} />
      {installState ? <RuntimeInstallStatePanel install={installState} /> : null}
      {runtime.logExcerpt ? (
        <div className="mt-3">
          <EvidenceDisclosure title="Runner diagnostics" body={runtime.logExcerpt} />
        </div>
      ) : null}
    </section>
  );
}
