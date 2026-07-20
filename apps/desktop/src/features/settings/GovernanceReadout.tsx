import { Cloud, FilePenLine, GitCommit, PlugZap, ShieldCheck, TerminalSquare } from "../../design/icons";

const rules = [
  {
    icon: <FilePenLine size={16} />,
    title: "File changes wait for you",
    body: "Agents ask before editing files in the repository.",
  },
  {
    icon: <TerminalSquare size={16} />,
    title: "Command runs wait for you",
    body: "Shell commands are shown before they run.",
  },
  {
    icon: <GitCommit size={16} />,
    title: "Git actions wait for you",
    body: "Commits and pushes are always deliberate.",
  },
  {
    icon: <Cloud size={16} />,
    title: "Cloud model use is shown first",
    body: "DesktopLab asks before sending repository context to a provider.",
  },
  {
    icon: <PlugZap size={16} />,
    title: "Community plugins start unverified",
    body: "Tool bridges and agent bridges stay separate until trust is approved.",
  },
  {
    icon: <ShieldCheck size={16} />,
    title: "Protected data stays on this device",
    body: "Secrets, environment files and credentials are kept local unless you explicitly override it.",
  },
];

export function GovernanceReadout() {
  return (
    <section className="border-t border-line py-4" aria-labelledby="governance-title">
      <div>
        <h2 id="governance-title" className="text-lg font-semibold">
          Trust defaults
        </h2>
        <p className="mt-1 text-sm leading-6 text-muted">Current safety defaults shown as read-only product behavior.</p>
      </div>

      <div className="mt-4 grid border-y border-line md:grid-cols-2">
        {rules.map((rule) => (
          <div key={rule.title} className="flex min-h-20 gap-3 border-b border-line px-1 py-3 md:odd:border-r md:[&:nth-last-child(-n+2)]:border-b-0">
            <div className="grid h-8 w-8 shrink-0 place-items-center text-muted">{rule.icon}</div>
            <div>
              <div className="text-sm font-semibold">{rule.title}</div>
              <p className="mt-1 text-sm leading-5 text-muted">{rule.body}</p>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
