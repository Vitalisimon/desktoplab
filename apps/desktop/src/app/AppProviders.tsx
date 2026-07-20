import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ApiProvider } from "../api/ApiProvider";
import { DesktopLabApiClient } from "../api/client";
import { createDesktopLabApiClient } from "./localApiConfig";
import { ThemeProvider } from "./theme";

type AppProvidersProps = {
  children: ReactNode;
  apiClient?: DesktopLabApiClient;
};

export function AppProviders({ children, apiClient }: AppProvidersProps) {
  const [resolvedClient, setResolvedClient] = useState<DesktopLabApiClient | null>(
    () => apiClient ?? null,
  );
  const [bootError, setBootError] = useState<string | null>(null);
  const queryClient = useMemo(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: 1,
            staleTime: 1_000,
            refetchOnWindowFocus: false,
          },
        },
      }),
    [],
  );
  useEffect(() => {
    let cancelled = false;
    if (apiClient) {
      setResolvedClient(apiClient);
      return;
    }
    createDesktopLabApiClient()
      .then((client) => {
        if (!cancelled) {
          setBootError(null);
          setResolvedClient(client);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setBootError(error instanceof Error ? error.message : "DesktopLab could not start.");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [apiClient]);

  if (bootError) {
    return <div data-testid="desktoplab-boot-error">{bootError}</div>;
  }

  if (!resolvedClient) {
    return <div data-testid="desktoplab-boot">Starting DesktopLab</div>;
  }

  return (
    <ApiProvider client={resolvedClient}>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>{children}</ThemeProvider>
      </QueryClientProvider>
    </ApiProvider>
  );
}
