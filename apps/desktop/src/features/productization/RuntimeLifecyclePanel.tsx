import type { RuntimeInventoryItem } from "../../api/types";

export function RuntimeLifecyclePanel({ runtime }: { runtime: RuntimeInventoryItem }) {
  if (!runtime.lifecycle) return null;

  return (
    <div className="mt-3 grid gap-2">
      <div className="rounded-desktop bg-elevated px-3 py-2">
        <p className="text-sm leading-5 text-muted">
          DesktopLab app updates are separate from local runner installs. Runner installs stay on
          demand and never request administrator access silently.
        </p>
      </div>
      {runtime.provenance ? (
        <LifecycleLine
          label="Verification"
          reason={`${runtime.provenance.verificationMethod}. Integrity: ${runtime.provenance.integrity.state}.`}
          stateLabel={runtime.provenance.installSource}
        />
      ) : null}
      <LifecycleLine label="Update" reason={runtime.lifecycle.update.reason} stateLabel={runtime.lifecycle.update.label} />
      <LifecycleLine label="Remove" reason={runtime.lifecycle.uninstall.reason} stateLabel={runtime.lifecycle.uninstall.label} />
    </div>
  );
}

function LifecycleLine({ label, reason, stateLabel }: { label: string; reason: string; stateLabel: string }) {
  return (
    <div className="rounded-desktop bg-elevated px-3 py-2">
      <p className="text-sm font-semibold text-ink">
        {label}: {stateLabel}
      </p>
      <p className="mt-1 text-sm leading-5 text-muted">{reason}</p>
    </div>
  );
}
