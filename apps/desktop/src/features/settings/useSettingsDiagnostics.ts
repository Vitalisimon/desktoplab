import { useQuery } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";

export function useSettingsDiagnostics() {
  const api = useApiClient();
  const retry = 3;
  const retryDelay = 250;
  const health = useQuery({ queryKey: ["settings", "health"], queryFn: () => api.health(), retry, retryDelay });
  const readiness = useQuery({ queryKey: ["settings", "readiness"], queryFn: () => api.readiness(), retry, retryDelay });
  const version = useQuery({ queryKey: ["settings", "version"], queryFn: () => api.version(), retry, retryDelay });
  const setup = useQuery({ queryKey: ["settings", "setup-preview"], queryFn: () => api.setupPreview(), retry, retryDelay });
  const diagnostics = useQuery({ queryKey: ["settings", "diagnostics"], queryFn: () => api.diagnostics(), retry, retryDelay });

  return {
    health,
    readiness,
    version,
    setup,
    diagnostics,
    isLoading: health.isLoading || readiness.isLoading || version.isLoading || setup.isLoading || diagnostics.isLoading,
    isError: health.isError || readiness.isError || version.isError || setup.isError || diagnostics.isError,
  };
}
