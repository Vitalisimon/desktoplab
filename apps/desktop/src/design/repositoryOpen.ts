export type RepositoryOpenTarget = {
  id: string;
  label: string;
  kind: "file_manager" | "ide";
};

export async function openRepositoryInFileManager(path: string) {
  if (typeof window === "undefined") return;
  if (!("__TAURI_INTERNALS__" in window)) return;

  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_repository_in_file_manager", { path });
}

export async function repositoryOpenTargets(): Promise<RepositoryOpenTarget[]> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RepositoryOpenTarget[]>("repository_open_targets");
}

export async function openRepositoryInTarget(path: string, targetId: string) {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_repository_in_target", { path, targetId });
}
