import { useEffect, useState } from "react";
import type { WorkspaceSnapshot } from "../api/types";
import { detectDesktopPlatform } from "./desktopPlatform";
import { openRepositoryInTarget, repositoryOpenTargets, type RepositoryOpenTarget } from "./repositoryOpen";

export function useRepositoryOpenTargets(workspace: WorkspaceSnapshot | null) {
  const [targets, setTargets] = useState<RepositoryOpenTarget[]>(() => [nativeFileManagerTarget()]);

  useEffect(() => {
    if (!workspace) {
      setTargets([nativeFileManagerTarget()]);
      return;
    }
    void repositoryOpenTargets()
      .then((discovered) => setTargets(discovered.length > 0 ? discovered : [nativeFileManagerTarget()]))
      .catch(() => setTargets([nativeFileManagerTarget()]));
  }, [workspace?.workspaceId]);

  return {
    targets,
    openTarget: (targetId: string) => {
      if (workspace) void openRepositoryInTarget(workspace.rootPath, targetId);
    },
  };
}

function nativeFileManagerTarget(): RepositoryOpenTarget {
  const platform = detectDesktopPlatform();
  const label = platform === "macos" ? "Finder" : platform === "windows" ? "File Explorer" : "File manager";
  return { id: "file_manager", label, kind: "file_manager" };
}
