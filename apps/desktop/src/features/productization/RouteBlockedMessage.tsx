import type { AgentWorkspaceSnapshot } from "../../api/types";

export function RouteBlockedMessage({ route, onOpenSetup }: { route: NonNullable<AgentWorkspaceSnapshot["route"]>; onOpenSetup: () => void }) {
  const reasons = route.blockedReasons?.length ? route.blockedReasons : route.reasons;
  return (
    <div className="mb-7 max-w-2xl text-[15px] leading-7 text-muted">
      <p className="text-ink">{route.summary}</p>
      {reasons.length > 0 ? (
        <ul className="mt-2 space-y-1">
          {reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      ) : null}
      {route.nextActionLabel ? (
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <p className="text-ink">Next: {route.nextActionLabel}</p>
          <button type="button" className="h-8 rounded-full border border-line bg-panel px-3 text-xs font-medium text-ink hover:bg-muted/10" onClick={onOpenSetup}>{route.nextActionLabel}</button>
        </div>
      ) : null}
    </div>
  );
}
