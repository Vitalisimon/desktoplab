import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useApiClient } from "../../api/ApiProvider";
import type { ExternalBackendSummary, PluginSummary } from "../../api/types";
import { CapabilityList, RouteExplanation, TrustBadge } from "../../design/OperationalPrimitives";
import { displayExternalBackendBoundary, displayPluginCompatibility } from "../../domain/displayNames";

export function ExtensionsFeature() {
  const api = useApiClient();
  const queryClient = useQueryClient();
  const plugins = useQuery({ queryKey: ["plugins"], queryFn: () => api.listPlugins() });
  const backends = useQuery({ queryKey: ["external-backends"], queryFn: () => api.listExternalBackends() });
  const trust = useMutation({
    mutationFn: (pluginId: string) => api.trustPlugin(pluginId, { decision: "approve" }),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["plugins"] }),
  });
  const route = useMutation({ mutationFn: (routeId: string) => api.approveExternalBackendRoute(routeId, { resolution: "deny" }) });

  if (plugins.isLoading || backends.isLoading) return <Panel title="Loading extensions" body="DesktopLab is reading extension readiness." />;
  if (plugins.isError || backends.isError || !plugins.data || !backends.data) {
    return <Panel title="Extensions unavailable" body="DesktopLab could not read extension readiness right now." />;
  }

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4">
      <div>
        <h1 className="text-2xl font-semibold">Extensions</h1>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted">Manage optional integrations without giving them trust silently.</p>
      </div>
      <section className="grid gap-4 lg:grid-cols-[0.95fr_1.05fr]">
        <div className="grid content-start gap-3">
          <h2 className="text-lg font-semibold">Plugins</h2>
          {plugins.data.plugins.map((plugin) => (
            <PluginRow key={plugin.pluginId} plugin={plugin} onTrust={() => trust.mutate(plugin.pluginId)} />
          ))}
        </div>
        <div className="grid content-start gap-3">
          <h2 className="text-lg font-semibold">External agents</h2>
          {backends.data.backends.map((backend) => (
            <BackendRow key={backend.backendId} backend={backend} onDeny={(routeId) => route.mutate(routeId)} />
          ))}
        </div>
      </section>
    </div>
  );
}

function PluginRow({ plugin, onTrust }: { plugin: PluginSummary; onTrust: () => void }) {
  const trustAction = plugin.trustActions.find((action) => action.id === "trust");
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-ink">{plugin.displayName}</p>
          <p className="mt-1 text-sm text-muted">{displayPluginCompatibility(plugin)}</p>
        </div>
        <TrustBadge trust={plugin.trust === "verified" ? "verified" : "unverified"} />
      </div>
      <div className="mt-3">
        <CapabilityList capabilities={plugin.capabilities} />
      </div>
      <PluginDistributionBoundary plugin={plugin} />
      <PluginProvenance plugin={plugin} />
      {plugin.blockedReasons.length > 0 ? (
        <ul className="mt-3 space-y-1">
          {plugin.blockedReasons.map((reason) => (
            <li key={reason} className="text-sm text-muted">{reason}</li>
          ))}
        </ul>
      ) : null}
      <div className="mt-3 flex gap-2">
        <button type="button" className="rounded-desktop bg-ink px-3 py-2 text-sm font-medium text-canvas disabled:opacity-45" disabled={!trustAction} onClick={onTrust}>
          Trust {plugin.displayName}
        </button>
      </div>
    </section>
  );
}

function PluginDistributionBoundary({ plugin }: { plugin: PluginSummary }) {
  if (!plugin.executionBoundary) return null;

  return (
    <div className="mt-3 rounded-desktop bg-elevated px-3 py-2">
      <p className="text-sm font-semibold text-ink">{plugin.executionBoundary.kind}</p>
      <p className="mt-1 text-sm leading-5 text-muted">{plugin.executionBoundary.reason}</p>
    </div>
  );
}

function PluginProvenance({ plugin }: { plugin: PluginSummary }) {
  if (!plugin.provenance) return null;
  const rows = [
    ["Descriptor", plugin.provenance.descriptorState],
    ["Runtime", plugin.provenance.runtimeRegistration],
    ["Integrity", plugin.provenance.integrityStatus],
    ["Execution", plugin.provenance.executionEligibility],
  ];
  return (
    <dl className="mt-3 grid grid-cols-2 gap-2 rounded-desktop bg-elevated px-3 py-2 text-sm">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt className="text-xs font-semibold uppercase text-muted">{label}</dt>
          <dd className="mt-1 text-ink">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function BackendRow({ backend, onDeny }: { backend: ExternalBackendSummary; onDeny: (routeId: string) => void }) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-ink">{backend.displayName}</p>
          <p className="mt-1 text-sm text-muted">{displayExternalBackendBoundary(backend)}</p>
        </div>
        <TrustBadge trust={backend.trust ?? "unverified"} />
      </div>
      <div className="mt-3">
        <CapabilityList capabilities={backend.capabilities} />
      </div>
      {backend.routes?.length ? (
        <div className="mt-3 grid gap-3">
          {backend.routes.map((route) => (
            <div key={route.routeId} className="grid gap-2">
              <RouteExplanation kind="external" summary={route.sessionOwnership ?? route.reason} reasons={route.blockedReasons?.length ? route.blockedReasons : [route.reason]} />
              {route.approvalRequired ? (
                <button type="button" className="rounded-desktop border border-line px-3 py-2 text-sm font-medium" onClick={() => onDeny(route.routeId)}>
                  Deny external route
                </button>
              ) : null}
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function Panel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm text-muted">{body}</p>
    </section>
  );
}
