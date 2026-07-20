import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { ApprovalResolveRequest } from "../../api/types";

type ResolveInput = ApprovalResolveRequest & {
  approvalId: string;
};

export function useApprovals() {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const queryKey = ["approvals"];

  const query = useQuery({
    queryKey,
    queryFn: () => api.listApprovals(),
    retry: false,
  });

  const resolve = useMutation({
    mutationKey: ["approvals", "resolve"],
    mutationFn: ({ approvalId, resolution }: ResolveInput) => api.resolveApproval(approvalId, { resolution }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  const approvals = query.data?.approvals ?? [];

  return {
    query,
    approvals,
    pendingApprovals: approvals.filter((approval) => approval.state === "pending"),
    resolve,
    isLoading: query.isLoading,
    isError: query.isError,
  };
}
