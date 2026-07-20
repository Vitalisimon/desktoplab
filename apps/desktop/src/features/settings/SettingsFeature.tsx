import { useEffect, useRef, useState } from "react";
import { ProvidersFeature } from "../productization/ProvidersFeature";
import { AppearanceSettingsPanel } from "./AppearanceSettingsPanel";
import { GovernanceReadout } from "./GovernanceReadout";
import { DiagnosticsBundlePanel } from "./DiagnosticsBundlePanel";
import { LocalDiagnosticsPanel } from "./LocalDiagnosticsPanel";
import { ProductizationSettingsSummary } from "./ProductizationSettingsSummary";
import { SafetyApprovalSettings } from "./SafetyApprovalSettings";
import { UpdateStatusPanel } from "./UpdateStatusPanel";
import { useSettingsDiagnostics } from "./useSettingsDiagnostics";
import { HighEndLocalSetupPanel } from "../setup/HighEndLocalSetupPanel";
import { ChevronRight } from "../../design/icons";

export function SettingsFeature() {
  const diagnostics = useSettingsDiagnostics();

  if (diagnostics.setup.isLoading) {
    return <SettingsPanel title="Checking settings" body="DesktopLab is reading local setup data." />;
  }

  if (!diagnostics.setup.data) {
    return <SettingsPanel title="Settings unavailable" body="DesktopLab could not read local setup data right now." />;
  }

  return (
    <div data-ui-route="settings" data-ui-state="ready" className="mx-auto grid w-full max-w-6xl gap-4 pb-16">
      <section data-testid="control-surface-header" className="border-b border-line pb-4">
        <h1 className="text-2xl font-semibold">Settings</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          See what is active now, then open advanced sections only when you want to change setup, safety or updates.
        </p>
      </section>

      <SettingsGroup title="Current setup" showHeading={false}>
        <ProductizationSettingsSummary />
      </SettingsGroup>

      <SettingsGroup title="Appearance" showHeading={false}>
        <AppearanceSettingsPanel />
      </SettingsGroup>

      {diagnostics.setup.data.highEndLocal?.status === "candidate" ? (
        <SettingsDisclosure title="High-capacity local route" description="Tune runtimes and models for large-memory hardware.">
          <HighEndLocalSetupPanel setup={diagnostics.setup.data.highEndLocal} embedded />
        </SettingsDisclosure>
      ) : null}

      <SettingsDisclosure title="Safety and approvals" description="Choose when agents pause before local actions.">
        <div className="grid gap-4">
          <SafetyApprovalSettings />
          <GovernanceReadout />
        </div>
      </SettingsDisclosure>

      <SettingsDisclosure title="Providers" description="Connect optional local, cloud or external execution routes.">
        <ProvidersFeature compact />
      </SettingsDisclosure>

      <SettingsDisclosure title="Updates" description="Review the installed version and update-channel readiness.">
        {diagnostics.diagnostics.data ? (
          <UpdateStatusPanel updateStatus={diagnostics.diagnostics.data.updateStatus} />
        ) : (
          <SettingsPanel title="Updates unavailable" body="DesktopLab could not read update status right now." />
        )}
      </SettingsDisclosure>

      <SettingsDisclosure title="Diagnostics" description="Inspect local services and prepare a redacted support bundle.">
        {diagnostics.diagnostics.data && diagnostics.health.data && diagnostics.readiness.data && diagnostics.version.data ? (
          <div className="grid gap-4">
            <LocalDiagnosticsPanel
              health={diagnostics.health.data}
              readiness={diagnostics.readiness.data}
              version={diagnostics.version.data}
              stability={diagnostics.diagnostics.data.stability}
            />
            <DiagnosticsBundlePanel bundle={diagnostics.diagnostics.data.bundlePreview} />
          </div>
        ) : (
          <SettingsPanel title="Diagnostics unavailable" body="DesktopLab could not read diagnostics right now." />
        )}
      </SettingsDisclosure>
    </div>
  );
}

function SettingsGroup({ title, children, showHeading = true }: { title: string; children: React.ReactNode; showHeading?: boolean }) {
  return (
    <section data-testid="settings-group" aria-label={title} className="grid gap-3">
      {showHeading ? <h2 className="px-1 text-sm font-semibold text-muted">{title}</h2> : null}
      {children}
    </section>
  );
}

function SettingsDisclosure({ title, description, children }: { title: string; description: string; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!open || !sectionRef.current) return;
    const section = sectionRef.current;
    const align = () => section.scrollIntoView({ block: "start" });
    align();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(align);
    observer.observe(section);
    return () => observer.disconnect();
  }, [open]);

  return (
    <section ref={sectionRef} data-testid="settings-disclosure" aria-label={title} className="scroll-mt-6 border-t border-line">
      <button
        type="button"
        aria-label={title}
        aria-expanded={open}
        className="group flex w-full items-center gap-4 px-1 py-4 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent"
        onClick={() => setOpen((value) => !value)}
      >
        <span className="min-w-0 flex-1">
          <span className="block text-sm font-semibold text-ink">{title}</span>
          <span className="mt-1 block text-xs font-normal leading-5 text-muted">{description}</span>
        </span>
        <ChevronRight aria-hidden="true" className={`size-4 shrink-0 text-muted transition-transform ${open ? "rotate-90" : "group-hover:translate-x-0.5"}`} />
      </button>
      {open ? <div className="pb-5 pt-1">{children}</div> : null}
    </section>
  );
}

function SettingsPanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}
