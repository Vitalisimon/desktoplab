import { runtimeConsentRequirement } from "./runtimeConsent";
import type { SetupChoice } from "../../api/types";

export function RuntimeConsentControl({
  runtimeId,
  setupChoice,
  accepted,
  disabled = false,
  onChange,
}: {
  runtimeId?: string;
  setupChoice?: SetupChoice;
  accepted: boolean;
  disabled?: boolean;
  onChange: (accepted: boolean) => void;
}) {
  const requirement = runtimeConsentRequirement(runtimeId, setupChoice);
  if (!requirement) return null;
  return (
    <div className="rounded-desktop border border-warning/30 bg-warning/10 px-4 py-3">
      <label className="flex items-start gap-3 text-sm font-medium text-ink">
        <input
          aria-label={requirement.label}
          type="checkbox"
          checked={accepted}
          disabled={disabled}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span>
          <span className="block">{requirement.label}</span>
          <span className="mt-1 block text-xs font-normal leading-5 text-muted">{requirement.detail}</span>
        </span>
      </label>
    </div>
  );
}
