import type { SetupChoice, SetupJobProgressItem, SetupPipelineSnapshot } from "../../api/types";
import { displaySetupJobId } from "../../domain/displayNames";
import { setupFailureCopy } from "./setupFailureCopy";

export function jobFromStartedId(id: string): SetupJobProgressItem {
  return {
    id,
    label: displaySetupJobId(id),
    status: "queued",
    progressPercent: 0,
  };
}

export function jobsFromAcceptance(
  jobs: Array<{ jobId: string; kind: string; state: SetupJobProgressItem["status"]; blockedReason?: string }> | undefined,
  pipeline?: SetupPipelineSnapshot,
): SetupJobProgressItem[] {
  const mapped = (jobs ?? []).map((job) => ({
    id: job.jobId,
    label: displaySetupJobId(job.kind),
    status: job.state,
    progressPercent: job.state === "running" ? 5 : 0,
    nextAction: setupFailureCopy(job.blockedReason),
  }));
  return mapped.length > 0 ? mapped : jobsFromPipeline(pipeline);
}

export function jobFromRuntimeInstall(install: {
  jobId: string;
  state: string;
  blockedReason?: string | null;
  remediation?: string;
}): SetupJobProgressItem {
  return {
    id: install.jobId,
    label: displaySetupJobId("runtime.install"),
    status: setupStatus(install.state),
    progressPercent: install.state === "completed" ? 100 : install.state === "blocked" ? 0 : 20,
    nextAction: setupFailureCopy(install.remediation ?? install.blockedReason),
  };
}

export function jobFromModelDownload(download: {
  jobId: string;
  state: string;
  progressPercent?: number;
  blockedReason?: string;
  failureReason?: string | null;
}): SetupJobProgressItem {
  return {
    id: download.jobId,
    label: displaySetupJobId("model.download"),
    status: setupStatus(download.state),
    progressPercent: download.progressPercent ?? (download.state === "ready" ? 100 : download.state === "blocked" ? 0 : 10),
    nextAction: setupFailureCopy(download.blockedReason ?? download.failureReason),
    cancelAvailable: download.state === "running" || download.state === "downloading",
  };
}

export function mergeSetupJobs(current: SetupJobProgressItem[], incoming: SetupJobProgressItem[]): SetupJobProgressItem[] {
  const incomingGroups = new Set(incoming.map(setupJobGroup));
  return [...current.filter((job) => !incomingGroups.has(setupJobGroup(job))), ...incoming];
}

export function uniqueJobIds(ids: Array<string | undefined>): string[] {
  return Array.from(new Set(ids.filter((id): id is string => Boolean(id))));
}

export function selectedId(items: Array<{ manifestId: string; role?: "recommended" | "alternative" }>, selected?: string): string | undefined {
  return (items.find((item) => item.manifestId === selected) ?? items.find((item) => item.role === "recommended") ?? items[0])?.manifestId;
}

export function setupChoiceFor(
  item: { setupChoiceRequired?: boolean; defaultSetupChoice?: SetupChoice } | undefined,
  choice: SetupChoice,
): SetupChoice | undefined {
  if (!item?.setupChoiceRequired) return undefined;
  return choice ?? item.defaultSetupChoice ?? "use_existing";
}

export function jobsFromPipeline(pipeline?: SetupPipelineSnapshot): SetupJobProgressItem[] {
  if (!pipeline || pipeline.state === "not_started" || pipeline.state === "ready") return [];
  if (pipeline.state === "model_downloading" || pipeline.state === "model_verifying") {
    return [
      {
        id: `pipeline.${pipeline.state}`,
        label: "Model download",
        status: "running",
        progressPercent: pipeline.state === "model_verifying" ? 85 : 45,
        nextAction: setupFailureCopy(pipeline.blockedReason),
      },
    ];
  }
  return [
    {
      id: `pipeline.${pipeline.state}`,
      label: pipeline.state === "blocked" ? "Setup pipeline" : "Runtime install",
      status: pipeline.state === "blocked" ? "blocked" : pipeline.state === "selected" ? "queued" : "running",
      progressPercent: pipeline.state === "selected" ? 5 : pipeline.state === "blocked" ? 0 : 20,
      nextAction: setupFailureCopy(pipeline.blockedReason),
    },
  ];
}

function setupJobGroup(job: SetupJobProgressItem): string {
  if (job.label === displaySetupJobId("runtime.install") || job.label === "Runtime install") return "runtime.install";
  if (job.label === displaySetupJobId("model.download") || job.label === "Model download") return "model.download";
  return job.id;
}

function setupStatus(state: string): SetupJobProgressItem["status"] {
  if (state === "completed" || state === "ready") return "completed";
  if (state === "blocked" || state === "external_guided") return "blocked";
  if (state === "failed") return "failed";
  if (state === "cancelled") return "cancelled";
  if (state === "running" || state === "downloading" || state === "installing" || state === "verifying") return "running";
  return "queued";
}
