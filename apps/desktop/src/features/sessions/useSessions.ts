import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { SessionCreateRequest } from "../../api/types";

export function useSessions(workspaceId: string) {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const queryKey = ["sessions", workspaceId];

  const sessions = useQuery({
    queryKey,
    queryFn: () => api.listSessions(workspaceId),
    enabled: workspaceId.trim().length > 0,
    retry: false,
  });

  const create = useMutation({
    mutationKey: ["sessions", "create", workspaceId],
    mutationFn: (request: SessionCreateRequest) => api.createSession(request),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  return {
    query: sessions,
    sessions: sessions.data?.sessions ?? [],
    create,
    isLoading: sessions.isLoading,
    isError: sessions.isError,
  };
}
