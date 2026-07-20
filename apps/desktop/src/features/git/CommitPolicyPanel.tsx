export function CommitPolicyPanel() {
  return (
    <section className="rounded-desktop border border-line bg-panel p-4 shadow-sm">
      <p className="text-sm font-semibold text-muted">Commit approval</p>
      <h2 className="mt-1 text-lg font-semibold">Review before Git writes</h2>
      <p className="mt-2 text-sm leading-6 text-muted">
        Commit and push requests appear in Approvals before DesktopLab lets them run.
      </p>
    </section>
  );
}
