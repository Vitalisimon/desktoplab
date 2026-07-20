import { Cpu, HardDrive, MemoryStick, MonitorCog } from "../../design/icons";
import type { HardwareFact, SetupPlanPreview } from "../../api/types";
import { visibleHardwareWarningCopies } from "./hardwareWarningCopy";
import { useState } from "react";

type HardwareSummaryProps = {
  hardware: SetupPlanPreview["hardware"];
  warnings: string[];
  defaultOpen?: boolean;
};

export function HardwareSummary({ hardware, warnings, defaultOpen }: HardwareSummaryProps) {
  const warningCopy = visibleHardwareWarningCopies(warnings, hardware);
  const openByDefault = defaultOpen ?? warningCopy.length > 0;
  const [open, setOpen] = useState(openByDefault);
  const facts = visibleHardwareFacts(hardware);

  return (
    <details open={open} className="rounded-desktop border border-line p-4 dl-panel">
      <summary
        role="button"
        aria-label="Your computer details"
        onClick={(event) => {
          event.preventDefault();
          setOpen((current) => !current);
        }}
        className="cursor-pointer list-none"
      >
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold leading-6 tracking-normal">Your computer</h2>
            <p className="mt-1 max-w-2xl text-sm leading-6 text-muted">
              DesktopLab checks what can run well here and avoids choices that would make setup fail.
            </p>
          </div>
          <span className="shrink-0 rounded-full border border-line bg-elevated px-3 py-1 text-xs font-semibold text-muted">
            {openByDefault ? "Needs attention" : "Details"}
          </span>
        </div>
      </summary>

      <div className="mt-4 grid grid-cols-2 gap-3">
        {facts.map(({ icon, fact, suffix, valueLabel }) => (
          <HardwareFactRow key={fact.label} icon={icon} fact={fact} suffix={suffix} valueLabel={valueLabel} visible={open} />
        ))}
      </div>

      {warningCopy.length > 0 ? (
        <div className="space-y-3 rounded-desktop border border-warning/30 bg-warning/10 px-4 py-3 text-sm text-ink">
          {warningCopy.map((warning) => (
            <div key={warning.diagnosticCode}>
              <p style={{ display: open ? undefined : "none" }} className="font-semibold">
                {warning.title}
              </p>
              <p style={{ display: open ? undefined : "none" }} className="mt-1 leading-5 text-muted">
                {warning.impact}
              </p>
            </div>
          ))}
        </div>
      ) : null}
    </details>
  );
}

function visibleHardwareFacts(hardware: SetupPlanPreview["hardware"]) {
  const hasUnifiedMemory = typeof hardware.unifiedMemoryGb.value === "number" && hardware.unifiedMemoryGb.value > 0;
  const hasIntegratedGraphics = hardware.acceleratorKind?.value === "integrated";
  const facts = [
    { icon: <Cpu size={16} />, fact: hardware.cpu },
    { icon: <MemoryStick size={16} />, fact: hardware.ramGb, suffix: "GB" },
    hasUnifiedMemory ? null : { icon: <MonitorCog size={16} />, fact: hardware.gpu },
    hasUnifiedMemory || hasIntegratedGraphics ? null : { icon: <MonitorCog size={16} />, fact: hardware.vramGb, suffix: "GB" },
    hasUnifiedMemory ? {
      icon: <MemoryStick size={16} />,
      fact: hardware.unifiedMemoryGb,
      suffix: "GB",
      valueLabel: `${hardware.unifiedMemoryGb.value} GB shared memory`,
    } : null,
    { icon: <HardDrive size={16} />, fact: hardware.storageAvailableGb, suffix: "GB" },
  ];
  return facts.filter((fact): fact is NonNullable<(typeof facts)[number]> => Boolean(fact));
}

function HardwareFactRow({
  icon,
  fact,
  suffix,
  valueLabel,
  visible,
}: {
  icon: React.ReactNode;
  fact: HardwareFact;
  suffix?: string;
  valueLabel?: string;
  visible: boolean;
}) {
  const value = valueLabel ?? (fact.value === null ? "Unknown" : `${fact.value}${suffix ? ` ${suffix}` : ""}`);
  return (
    <div className="flex min-h-16 items-center gap-3 rounded-desktop border border-line px-4 dl-elevated">
      <div className="grid h-8 w-8 shrink-0 place-items-center rounded-desktop bg-accent/10 text-accent">{icon}</div>
      <div className="min-w-0 flex-1">
        <div className="text-xs font-medium uppercase text-muted">{fact.label}</div>
        <div style={{ display: visible ? undefined : "none" }} className="truncate text-sm font-semibold text-ink">
          {value}
        </div>
      </div>
      <span className={`rounded px-2 py-1 text-[11px] font-medium ${confidenceClass(fact.confidence)}`}>
        {confidenceLabel(fact.confidence)}
      </span>
    </div>
  );
}

function confidenceLabel(confidence: HardwareFact["confidence"]) {
  if (confidence === "confirmed") return "Checked";
  if (confidence === "unknown") return "Needs check";
  if (confidence === "unsupported") return "Unsupported";
  if (confidence === "conflicting") return "Review";
  return "Likely";
}

function confidenceClass(confidence: HardwareFact["confidence"]) {
  if (confidence === "confirmed") return "bg-success/10 text-success";
  if (confidence === "unknown") return "bg-elevated text-muted";
  if (confidence === "unsupported" || confidence === "conflicting") return "bg-danger/10 text-danger";
  return "bg-warning/10 text-warning";
}
