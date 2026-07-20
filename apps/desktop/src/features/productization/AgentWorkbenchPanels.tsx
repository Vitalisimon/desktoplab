const emptyWorkbenchActions = [
  { label: "Explain this project", prompt: "Explain this project and summarize how it is structured." },
  { label: "Review changes", prompt: "Review the current repository changes and highlight risks." },
  { label: "Find setup instructions", prompt: "Find setup instructions and tell me how to run the project." },
  { label: "Prepare a task plan", prompt: "Prepare a task plan for the next change in this repository." },
] as const;

export function EmptyWorkbenchActions({ repositoryReady, onChoose }: { repositoryReady: boolean; onChoose: (prompt: string) => void }) {
  return (
    <div className="flex min-h-full items-end pb-8">
      <div className="grid gap-4">
        <p className="max-w-2xl text-[15px] leading-7 text-muted">Ask DesktopLab what to change, inspect, or verify in this repository.</p>
        {repositoryReady ? (
          <div className="flex flex-wrap gap-2" aria-label="Suggested repository prompts">
            {emptyWorkbenchActions.map((action) => (
              <button key={action.label} type="button" className="h-8 rounded-full border border-line bg-panel px-3 text-xs font-medium text-muted hover:text-ink" onClick={() => onChoose(action.prompt)}>
                {action.label}
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
