import { createContext, useContext, type ReactNode } from "react";
import { DesktopLabApiClient } from "./client";

const ApiClientContext = createContext<DesktopLabApiClient | null>(null);

export function ApiProvider({ client, children }: { client: DesktopLabApiClient; children: ReactNode }) {
  return <ApiClientContext.Provider value={client}>{children}</ApiClientContext.Provider>;
}

export function useApiClient() {
  const client = useContext(ApiClientContext);
  if (!client) {
    throw new Error("DesktopLab API client provider is missing");
  }
  return client;
}
