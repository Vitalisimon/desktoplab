import { useMutation } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";

export function useWorkspaceOpen() {
  const api = useApiClient();
  const open = useMutation({
    mutationKey: ["workspaces", "open"],
    mutationFn: (request: { path: string }) => api.openWorkspace(request),
  });

  return {
    open,
    workspace: open.data,
    isOpening: open.isPending,
    error: open.error,
  };
}
