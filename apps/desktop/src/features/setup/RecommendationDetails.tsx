import { ChevronDown, Sparkles } from "../../design/icons";
import type { ReactNode } from "react";
import type { SetupPlanPreview, SetupRecommendation } from "../../api/types";
import { InstalledEvidence, ModelMetadata, ModelMetadataLine, ModelOutcomeCopy, RawModelDetails } from "./ModelMetadata";
import {
  friendlyChannel,
  friendlyCompatibilityReason,
  friendlyHiddenReason,
  friendlyInstallMode,
  registryLabel,
} from "./recommendationLabels";
import { visibleExpectedLimitations } from "./expectedLimitationCopy";

export function SelectableAlternatives({
  title,
  items,
  onSelect,
}: {
  title: string;
  items: SetupRecommendation[];
  onSelect?: (manifestId: string) => void;
}) {
  const groups = groupRecommendationsByFamily(items);
  return (
    <details className="rounded-desktop border border-line px-4 py-3 text-sm dl-panel">
      <summary className="flex cursor-pointer list-none items-center justify-between font-medium">
        {title}
        <ChevronDown size={15} />
      </summary>
      {groups.length > 1 ? <p className="mt-2 text-xs leading-5 text-muted">More local model families are available as this catalog grows.</p> : null}
      <div className="mt-3 grid gap-3 text-muted">
        {groups.map((group) => (
          <section key={group.name} aria-label={`${group.name} options`} className="grid gap-2">
            {group.showHeading ? <h3 className="text-xs font-semibold uppercase text-muted">{group.name}</h3> : null}
            <ul className="grid gap-2">
              {group.items.map((alternative) => (
                <li key={alternative.manifestId} className="rounded-desktop px-3 py-2 dl-elevated">
                  <div className="flex items-start justify-between gap-3">
                    <span className="min-w-0">
                      <span className="block truncate font-medium text-ink">{alternative.displayName}</span>
                      {alternative.installMode ? <span className="mt-0.5 block text-xs text-muted">{friendlyInstallMode(alternative.installMode)}</span> : null}
                      <ModelMetadataLine recommendation={alternative} />
                      <ModelOutcomeCopy recommendation={alternative} />
                    </span>
                    {isSelectableForLocalSetup(alternative) ? (
                      <button type="button" className="rounded bg-panel px-2 py-1 text-xs font-semibold text-ink transition-colors duration-150 hover:bg-accent hover:text-white" onClick={() => onSelect?.(alternative.manifestId)}>
                        Select {alternative.displayName}
                      </button>
                    ) : (
                      <span className="rounded bg-panel px-2 py-1 text-xs font-semibold text-muted">
                        {alternative.parameterClass === "cloud" ? "Connect provider" : "Guided setup"}
                      </span>
                    )}
                  </div>
                  <RawModelDetails recommendation={alternative} />
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    </details>
  );
}

export function RecommendationCard({
  icon,
  title,
  recommendation,
}: {
  icon: ReactNode;
  title: string;
  recommendation?: SetupRecommendation;
}) {
  return (
    <div className="rounded-desktop border border-line p-4 dl-panel">
      <div className="flex items-center gap-2 text-sm font-medium text-muted">
        {icon}
        {title}
      </div>
      <div className="mt-3 text-base font-semibold">{recommendation?.displayName ?? "Not available"}</div>
      {recommendation?.familyName ? <div className="mt-1 text-xs font-semibold uppercase text-muted">{recommendation.familyName}</div> : null}
      <div className="mt-1 text-xs text-muted">{recommendation ? "Selected for this computer" : "No compatible option"}</div>
      {recommendation?.hostInstallState === "installed" ? (
        <div className="mt-2 rounded bg-success/10 px-2 py-1.5 text-xs font-semibold text-success">
          <div>Already installed</div>
          <InstalledEvidence recommendation={recommendation} />
        </div>
      ) : null}
      {recommendation?.setupChoiceRequired ? (
        <div className="mt-2 text-xs font-medium text-muted">
          DesktopLab uses the existing local install by default.
        </div>
      ) : null}
      {recommendation?.compatibilityReason ? (
        <div className="mt-2 text-xs font-medium text-muted">{friendlyCompatibilityReason(recommendation.compatibilityReason)}</div>
      ) : null}
      {recommendation?.installMode ? <div className="mt-2 text-xs font-medium text-muted">{friendlyInstallMode(recommendation.installMode)}</div> : null}
      {recommendation ? <ModelOutcomeCopy recommendation={recommendation} /> : null}
      {recommendation ? <ModelMetadata recommendation={recommendation} /> : null}
      {recommendation ? <RawModelDetails recommendation={recommendation} /> : null}
      {recommendation ? (
        <div className="mt-3 inline-flex rounded bg-elevated px-2 py-1 text-[11px] font-medium text-muted">
          {friendlyChannel(recommendation.channel)}
        </div>
      ) : null}
    </div>
  );
}

export function ExpectedLimitations({ limitations }: { limitations: string[] }) {
  const visibleLimitations = visibleExpectedLimitations(limitations);
  if (visibleLimitations.length === 0) return null;
  return (
    <div className="rounded-desktop border border-line px-4 py-3 dl-panel">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Sparkles size={15} />
        Expected limitations
      </div>
      <ul className="mt-2 space-y-1 text-sm leading-6 text-muted">
        {visibleLimitations.map((limitation) => (
          <li key={limitation}>{limitation}</li>
        ))}
      </ul>
    </div>
  );
}

export function HiddenReasonDetails({ title, reasons }: { title: string; reasons: string[] }) {
  if (reasons.length === 0) return null;
  return (
    <details className="rounded-desktop border border-line px-4 py-3 text-sm dl-panel">
      <summary className="flex cursor-pointer list-none items-center justify-between font-medium">
        {title}
        <ChevronDown size={15} />
      </summary>
      <ul className="mt-2 space-y-1 text-muted">
        {reasons.map((reason) => (
          <li key={reason}>{friendlyHiddenReason(reason)}</li>
        ))}
      </ul>
    </details>
  );
}

export function RegistryState({ state }: { state: SetupPlanPreview["registryState"] }) {
  const className =
    state === "ready"
      ? "bg-success/10 text-success"
      : state === "degraded"
        ? "bg-warning/10 text-warning"
        : "bg-danger/10 text-danger";
  return <span className={`rounded px-2 py-1 text-xs font-medium ${className}`}>{registryLabel(state)}</span>;
}

export function selectedRecommendation(items: SetupRecommendation[], selectedId?: string) {
  return items.find((item) => item.manifestId === selectedId) ?? items.find((item) => item.role === "recommended") ?? items[0];
}

export function isSelectableForLocalSetup(item?: SetupRecommendation) {
  if (!item) return false;
  if (item.parameterClass === "cloud") return false;
  if (item.runtimeId && item.runtimeId.includes("cloud")) return false;
  if (item.installMode === "external_guided") return false;
  return true;
}

type RecommendationGroup = {
  name: string;
  showHeading: boolean;
  items: SetupRecommendation[];
};

function groupRecommendationsByFamily(items: SetupRecommendation[]): RecommendationGroup[] {
  const groups = new Map<string, SetupRecommendation[]>();
  for (const item of items) {
    const name = item.familyName ?? "Compatible";
    groups.set(name, [...(groups.get(name) ?? []), item]);
  }
  return Array.from(groups, ([name, groupItems]) => ({
    name,
    showHeading: groups.size > 1 || name !== "Compatible",
    items: groupItems,
  }));
}
