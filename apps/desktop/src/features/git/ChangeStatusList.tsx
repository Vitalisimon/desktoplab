type ChangeStatusListProps = {
  entries: string[];
};

export function ChangeStatusList({ entries }: ChangeStatusListProps) {
  return (
    <section className="rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h2 className="text-lg font-semibold">Repository changes</h2>
      {entries.length === 0 ? (
        <p className="mt-3 rounded-desktop bg-elevated px-3 py-2 text-sm text-muted">No local changes reported.</p>
      ) : (
        <ul className="mt-3 space-y-2">
          {entries.map((entry) => (
            <li key={entry} className="rounded-desktop bg-elevated px-3 py-2 text-sm font-medium">
              {entry}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
