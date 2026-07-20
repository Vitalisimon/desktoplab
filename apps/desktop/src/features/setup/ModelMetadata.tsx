import type { SetupRecommendation } from "../../api/types";
import { friendlyCompatibilityReason, friendlyParameterClass, friendlyRuntimeId, licenseLabel } from "./recommendationLabels";

export function InstalledEvidence({ recommendation }: { recommendation: SetupRecommendation }) {
  const evidence = [
    recommendation.installedVersion ? `Version ${recommendation.installedVersion}` : null,
    recommendation.installedPath ?? null,
    recommendation.endpoint ?? null,
  ].filter((item): item is string => Boolean(item));
  if (evidence.length === 0) return null;
  return (
    <div className="mt-1 space-y-0.5 font-medium text-success/85">
      {evidence.map((item) => (
        <div key={item}>{item}</div>
      ))}
    </div>
  );
}

export function ModelMetadata({ recommendation }: { recommendation: SetupRecommendation }) {
  const items = metadataItems(recommendation);
  if (items.length === 0) return null;
  return (
    <div className="mt-3 flex flex-wrap gap-2">
      {items.map((item) => (
        <span key={item} className="rounded bg-elevated px-2 py-1 text-[11px] font-semibold text-muted">
          {item}
        </span>
      ))}
    </div>
  );
}

export function ModelOutcomeCopy({ recommendation }: { recommendation: SetupRecommendation }) {
  const items = outcomeItems(recommendation);
  if (items.length === 0) return null;
  return (
    <div className="mt-3 flex flex-wrap gap-2">
      {items.map((item) => (
        <span key={item} className="rounded bg-success/10 px-2 py-1 text-[11px] font-semibold text-success">
          {item}
        </span>
      ))}
    </div>
  );
}

export function ModelMetadataLine({ recommendation }: { recommendation: SetupRecommendation }) {
  const items = metadataItems(recommendation, true);
  if (items.length === 0) return null;
  return (
    <span className="mt-1 flex flex-wrap gap-1.5 text-xs text-muted">
      {items.map((item) => (
        <span key={item} className="rounded bg-panel px-1.5 py-0.5">
          {item}
        </span>
      ))}
    </span>
  );
}

export function RawModelDetails({ recommendation }: { recommendation: SetupRecommendation }) {
  const details = [
    recommendation.agentContextWindowTokens
      ? `Agent context on this computer ${recommendation.agentContextWindowTokens.toLocaleString()} tokens`
      : null,
    recommendation.contextWindowTokens ? `Model maximum ${recommendation.contextWindowTokens.toLocaleString()} tokens` : null,
    recommendation.agentRequestTimeoutSeconds
      ? `Local turn timeout ${Math.ceil(recommendation.agentRequestTimeoutSeconds / 60)} minutes`
      : null,
    recommendation.runtimeId ? `Runner ${friendlyRuntimeId(recommendation.runtimeId)}` : null,
  ].filter((item): item is string => Boolean(item));
  if (details.length === 0) return null;
  return (
    <details className="mt-2 text-xs text-muted">
      <summary role="button" className="cursor-pointer font-medium">Model and runner details</summary>
      <ul className="mt-1 space-y-0.5">
        {details.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </details>
  );
}

function metadataItems(recommendation: SetupRecommendation, diskLabel = false) {
  const cloudModel = isCloudModel(recommendation);
  return [
    recommendation.parametersBillion ? `${recommendation.parametersBillion}B` : null,
    cloudModel ? null : (recommendation.quantization ?? null),
    recommendation.requiredMemoryGb ? `${recommendation.requiredMemoryGb} GB memory class` : null,
    !cloudModel && recommendation.expectedDiskMb ? `~${(recommendation.expectedDiskMb / 1000).toFixed(1)} GB${diskLabel ? " on disk" : ""}` : null,
    recommendation.parameterClass ? friendlyParameterClass(recommendation.parameterClass) : null,
    recommendation.agentQualification === "runtime_validation_required"
      ? "Validated after download"
      : null,
    diskLabel ? null : (recommendation.trustLabel ?? licenseLabel(recommendation.licenseState)),
  ].filter((item): item is string => Boolean(item));
}

export function outcomeItems(recommendation: SetupRecommendation) {
  const cloudModel = isCloudModel(recommendation);
  return [
    cloudModel && recommendation.compatibilityReason ? friendlyCompatibilityReason(recommendation.compatibilityReason) : null,
    !cloudModel && recommendation.compatibilityReason ? "Recommended for this computer" : null,
    !cloudModel && recommendation.installMode !== "external_guided" ? "Works offline" : null,
    !cloudModel && recommendation.expectedDiskMb ? `Needs about ${(recommendation.expectedDiskMb / 1000).toFixed(1)} GB disk` : null,
  ].filter((item): item is string => Boolean(item));
}

function isCloudModel(recommendation: SetupRecommendation): boolean {
  return recommendation.parameterClass === "cloud" || recommendation.runtimeId === "runtime.ollama-cloud";
}
