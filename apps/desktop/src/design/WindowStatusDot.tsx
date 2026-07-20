export function WindowStatusDot({ hasWorkspace }: { hasWorkspace: boolean }) {
  const className = hasWorkspace ? "bg-accent shadow-[0_0_12px_rgb(var(--dl-color-accent)/0.45)]" : "bg-muted/50";
  return <span aria-hidden="true" className={`h-1.5 w-1.5 shrink-0 rounded-full ${className}`} />;
}
