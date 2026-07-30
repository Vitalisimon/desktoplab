import type { RuntimeSetupCapability } from "../../api/types";
import { runtimeCapabilityDetail, runtimeCapabilityLabel } from "./runtimeCapabilityCopy";

export function RuntimeCapabilityStatus({ capability }: { capability: RuntimeSetupCapability }) {
  return (
    <>
      <span className="mt-0.5 block text-xs font-medium text-muted">
        {runtimeCapabilityLabel(capability)}
      </span>
      <span className="mt-0.5 block text-xs text-muted">
        {runtimeCapabilityDetail(capability)}
      </span>
    </>
  );
}
