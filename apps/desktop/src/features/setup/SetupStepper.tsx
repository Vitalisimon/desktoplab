export function SetupStepper({ ready, hasProgress }: { ready: boolean; hasProgress: boolean }) {
  const steps = ["Detect", "Recommend", "Install", "Verify", "Open"];
  const activeIndex = ready ? 4 : hasProgress ? 2 : 1;
  return (
    <nav aria-label="Setup steps" className="rounded-desktop border border-line px-4 py-3 dl-panel">
      <ol className="grid grid-cols-5 gap-2">
        {steps.map((step, index) => {
          const complete = index < activeIndex || ready;
          const active = index === activeIndex && !ready;
          return (
            <li key={step} className="flex items-center gap-2 text-xs font-semibold text-muted">
              <span className={`grid h-6 w-6 place-items-center rounded-full border text-[11px] transition-colors duration-150 ${stepClassName(complete, active)}`}>
                {index + 1}
              </span>
              <span className={complete || active ? "text-ink" : undefined}>{step}</span>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

function stepClassName(complete: boolean, active: boolean) {
  if (complete) return "border-accent bg-accent text-white shadow-[var(--dl-accent-glow)]";
  if (active) return "border-accent bg-accent/10 text-accent";
  return "border-line bg-elevated text-muted";
}
