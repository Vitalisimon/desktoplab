import { useQuery } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { ProviderAccount } from "../../api/types";
import { CapabilityList, EvidenceDisclosure, RouteExplanation, TrustBadge } from "../../design/OperationalPrimitives";
import { displayCapability, displayProviderAccountMode, displayProviderAccountState } from "../../domain/displayNames";
import { ProviderConnectPanel, accountModesFor } from "./ProviderConnectPanel";

export function ProvidersFeature({ compact = false }: { compact?: boolean }) {
  const api = useApiClient();
  const providers = useQuery({ queryKey: ["providers"], queryFn: () => api.listProviders() });
  const route = useQuery({ queryKey: ["routing", "preference"], queryFn: () => api.routePreference() });

  if (providers.isLoading || route.isLoading) return <Panel title="Loading accounts" body="DesktopLab is reading account readiness." />;
  if (providers.isError || route.isError || !providers.data || !route.data) {
    return <Panel title="Accounts unavailable" body="DesktopLab could not read provider readiness right now." />;
  }

  return (
    <div className={`${compact ? "grid w-full gap-4" : "mx-auto grid w-full max-w-6xl gap-4"}`}>
      <div>
        {compact ? <h2 className="text-lg font-semibold">Accounts</h2> : <h1 className="text-2xl font-semibold">Accounts</h1>}
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Add optional model accounts. DesktopLab stays local-first and asks before sending repository context outside this machine.
        </p>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Cloud accounts are optional. Local models stay the default route until you choose otherwise.
        </p>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">
          Cloud models run only after you connect an account and approve the policy route.
        </p>
      </div>

      <section className="grid items-start gap-4 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="grid gap-3">
          {providers.data.providers.map((provider) => (
            <ProviderRow key={provider.providerId} provider={provider} />
          ))}
        </div>
        <div className="grid content-start gap-4">
          <RouteExplanation kind="cloud" summary={route.data.explanation || "Connect an account before using a cloud route."} reasons={route.data.blockedReasons ?? []} />
          <ProviderConnectPanel providers={providers.data.providers} />
        </div>
      </section>
    </div>
  );
}

function ProviderRow({ provider }: { provider: ProviderAccount }) {
  const state = displayProviderAccountState(provider);
  const detail = providerAccountDetail(provider);
  const accountMode = provider.activeAccountMode ?? accountModesFor(provider)[0];
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="text-sm font-semibold text-ink">{provider.displayName}</p>
          <p className="mt-1 text-sm leading-5 text-muted">{detail}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <span className={`rounded-full px-2 py-1 text-xs font-semibold ${state.className}`}>{state.label}</span>
          <TrustBadge trust={provider.trust} label={providerTrustLabel(provider.trust)} />
        </div>
      </div>
      <div className="mt-3">
        <CapabilityList capabilities={provider.capabilities} format={displayCapability} />
      </div>
      <p className="mt-3 text-sm text-muted">Account mode: {displayProviderAccountMode(accountMode)}</p>
      <p className="mt-2 text-sm text-muted">{providerEgressDetail(provider)}</p>
      <div className="mt-3">
        <EvidenceDisclosure title="Diagnostics" body={provider.diagnostic.redactedEvidence} />
      </div>
    </section>
  );
}

function providerTrustLabel(trust: ProviderAccount["trust"]): string {
  if (trust === "verified") return "Verified integration";
  if (trust === "local") return "Local integration";
  return "Unverified integration";
}

function providerAccountDetail(provider: ProviderAccount): string {
  if (provider.status === "connected") return "Ready for approved cloud routing.";
  if (provider.status === "missing_credential") {
    return "Connect this account only if you want DesktopLab to route selected work to it.";
  }
  if (provider.status === "degraded") return "DesktopLab needs your approval before this provider can be used.";
  if (provider.diagnostic.message.toLowerCase().includes("unavailable")) {
    return "This account route is unavailable on this machine or policy.";
  }
  return "Policy prevents this provider from receiving repository context.";
}

function providerEgressDetail(provider: ProviderAccount): string {
  if (provider.egress === "local_only") return "No data leaves this machine.";
  if (provider.egress === "allowed") return "Allowed by your current policy.";
  return "Asks before sending repository context.";
}

function Panel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}
