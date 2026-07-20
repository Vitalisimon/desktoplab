import { useMutation, useQuery } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { SetupAcceptanceRequest } from "../../api/types";

export function useSetupPreview() {
  const api = useApiClient();
  const preview = useQuery({
    queryKey: ["setup", "preview"],
    queryFn: () => api.setupPreview(),
    retry: 3,
    retryDelay: 250,
  });
  const accept = useMutation({
    mutationKey: ["setup", "accept"],
    mutationFn: (request: SetupAcceptanceRequest) => api.acceptSetupPlan(request),
  });

  return {
    preview,
    accept,
    isReady: preview.data?.registryState !== "blocked" && Boolean(preview.data?.runtimeRecommendations.length),
    isDegraded: preview.data?.registryState === "degraded",
    isBlocked: preview.data?.registryState === "blocked",
  };
}
