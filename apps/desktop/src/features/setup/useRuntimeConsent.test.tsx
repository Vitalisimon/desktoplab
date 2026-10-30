// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { ApiProvider } from "../../api/ApiProvider";
import type { DesktopLabApiClient } from "../../api/client";
import { useRuntimeConsent } from "./useRuntimeConsent";

test("remount restores only the explicitly accepted consent and permits revocation", async () => {
  const values = { mlx: false, lmStudio: false };
  const api = {
    runtimeConsents: vi.fn(async () => ({ consents: { ...values } })),
    setRuntimeConsent: vi.fn(async (id: "mlx" | "lmStudio", accepted: boolean) => {
      values[id] = accepted;
      return { consents: { ...values } };
    }),
  } as unknown as DesktopLabApiClient;
  const makeWrapper = () => {
    const client = new QueryClient();
    return ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}><ApiProvider client={api}>{children}</ApiProvider></QueryClientProvider>;
  };
  const wrapper = makeWrapper();
  const first = renderHook(() => useRuntimeConsent("runtime.mlx-lm"), { wrapper });
  await waitFor(() => expect(first.result.current.pending).toBe(false));
  expect(first.result.current.accepted).toBe(false);
  act(() => first.result.current.change(true));
  await waitFor(() => expect(first.result.current.accepted).toBe(true));
  first.unmount();
  const second = renderHook(({ id }) => useRuntimeConsent(id), { wrapper: makeWrapper(), initialProps: { id: "runtime.mlx-lm" } });
  await waitFor(() => expect(second.result.current.accepted).toBe(true));
  second.rerender({ id: "runtime.lm-studio" });
  expect(second.result.current.accepted).toBe(false);
  second.rerender({ id: "runtime.mlx-lm" });
  act(() => second.result.current.change(false));
  await waitFor(() => expect(second.result.current.accepted).toBe(false));
  expect(values.mlx).toBe(false);
});

test("storage failure never presents consent as accepted", async () => {
  const api = {
    runtimeConsents: vi.fn().mockResolvedValue({ consents: { mlx: false, lmStudio: false } }),
    setRuntimeConsent: vi.fn().mockRejectedValue(new Error("storage unavailable")),
  } as unknown as DesktopLabApiClient;
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={queryClient}><ApiProvider client={api}>{children}</ApiProvider></QueryClientProvider>;
  const hook = renderHook(() => useRuntimeConsent("runtime.mlx-lm"), { wrapper });
  await waitFor(() => expect(hook.result.current.pending).toBe(false));
  act(() => hook.result.current.change(true));
  await waitFor(() => expect(hook.result.current.error).toBe(true));
  expect(hook.result.current.accepted).toBe(false);
});
