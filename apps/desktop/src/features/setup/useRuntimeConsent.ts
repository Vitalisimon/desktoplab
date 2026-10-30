import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";

export function useRuntimeConsent(runtimeId?: string) {
  const api = useApiClient();
  const client = useQueryClient();
  const id = runtimeId === "runtime.mlx-lm" ? "mlx" : runtimeId === "runtime.lm-studio" ? "lmStudio" : undefined;
  const query = useQuery({ queryKey: ["runtime-consents"], queryFn: () => api.runtimeConsents(), enabled: Boolean(id), retry: false });
  const mutation = useMutation({
    mutationFn: ({ id, accepted }: { id: "mlx" | "lmStudio"; accepted: boolean }) => api.setRuntimeConsent(id, accepted),
    onSuccess: (data) => client.setQueryData(["runtime-consents"], data),
  });
  return {
    accepted: id ? query.data?.consents[id] === true : false,
    pending: Boolean(id) && (query.isLoading || mutation.isPending),
    error: Boolean(id) && (query.isError || mutation.isError),
    change: (accepted: boolean) => { if (id) mutation.mutate({ id, accepted }); },
  };
}
