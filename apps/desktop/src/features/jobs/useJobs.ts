import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import { reduceJobEvents } from "./jobEventReducer";

export function useJobs() {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const jobs = useQuery({
    queryKey: ["jobs"],
    queryFn: async () => {
      const response = await api.listJobs();
      const replayEvents = api.replayEvents;
      if (typeof replayEvents !== "function") return response;
      try {
        const frames = await replayEvents.call(api);
        return { jobs: reduceJobEvents(response.jobs, frames) };
      } catch {
        return response;
      }
    },
    retry: false,
    refetchInterval: 2_000,
  });
  const retry = useMutation({
    mutationKey: ["jobs", "retry"],
    mutationFn: (jobId: string) => api.retryJob(jobId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["jobs"] });
    },
  });

  return {
    query: jobs,
    jobs: jobs.data?.jobs ?? [],
    retry,
    isLoading: jobs.isLoading,
    isError: jobs.isError,
  };
}
