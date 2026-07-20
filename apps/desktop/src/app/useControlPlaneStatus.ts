import { useQuery } from "@tanstack/react-query";
import { useApiClient } from "../api/ApiProvider";

export function useControlPlaneStatus() {
  const api = useApiClient();
  const health = useQuery({
    queryKey: ["control-plane", "health"],
    queryFn: () => api.health(),
  });
  const readiness = useQuery({
    queryKey: ["control-plane", "readiness"],
    queryFn: () => api.readiness(),
  });

  return {
    health,
    readiness,
    isLoading: health.isLoading || readiness.isLoading,
    isDegraded: readiness.data?.state === "degraded",
    isBlocked: readiness.data?.state === "blocked",
  };
}
