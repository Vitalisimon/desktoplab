import { JobCenterView } from "./JobCenterView";
import { useJobs } from "./useJobs";

export function JobsFeature() {
  const jobs = useJobs();

  if (jobs.isLoading) {
    return <JobsPanel title="Loading background work" body="DesktopLab is checking current setup activity." />;
  }

  if (jobs.isError) {
    return <JobsPanel title="Background work unavailable" body="DesktopLab could not read setup activity right now." />;
  }

  return (
    <JobCenterView
      jobs={jobs.jobs}
      onRetry={(jobId) => jobs.retry.mutate(jobId)}
      retryingJobId={typeof jobs.retry.variables === "string" && jobs.retry.isPending ? jobs.retry.variables : null}
    />
  );
}

function JobsPanel({ title, body }: { title: string; body: string }) {
  return (
    <section className="mx-auto max-w-3xl rounded-desktop border border-line bg-panel p-5 shadow-sm">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="mt-2 text-sm leading-6 text-muted">{body}</p>
    </section>
  );
}
