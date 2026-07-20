import type { AppStateResponse, WorkspaceSnapshot } from "./types";

export type DrawerPinnedItemType = "project" | "thread";

export type DrawerPinnedItem = {
  id: string;
  type: DrawerPinnedItemType;
  label: string;
};

const drawerPinnedItemsKey = "desktoplab.drawer.pinnedItems";

export const defaultAppState: AppStateResponse = {
  readiness: { state: "degraded" },
  setup: { state: "not_started" },
  setupPipeline: { state: "not_started" },
  currentWorkspace: null,
  workspaces: [],
  routeInput: {
    readiness: "degraded",
    setupState: "not_started",
    hasWorkspace: false,
    activeApprovalCount: 0,
    activeSessionCount: 0,
  },
};

export function appStateWithOpenedWorkspace(
  current: AppStateResponse | undefined,
  workspace: WorkspaceSnapshot,
): AppStateResponse {
  const base = current ?? defaultAppState;
  return {
    ...base,
    readiness: { state: "ready" },
    setup: { state: "ready" },
    setupPipeline: { state: "ready" },
    currentWorkspace: workspace,
    workspaces: mergeWorkspaces(base.workspaces ?? [], workspace),
    routeInput: {
      ...base.routeInput,
      readiness: "ready",
      setupState: "ready",
      hasWorkspace: true,
    },
  };
}

function mergeWorkspaces(workspaces: WorkspaceSnapshot[], workspace: WorkspaceSnapshot): WorkspaceSnapshot[] {
  const rest = workspaces.filter((item) => item.workspaceId !== workspace.workspaceId);
  return [...rest, workspace];
}

export function readDrawerPinnedItems(storage: Storage | null = browserStorage()): DrawerPinnedItem[] {
  if (!storage) return [];
  const raw = storage.getItem(drawerPinnedItemsKey);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isDrawerPinnedItem);
  } catch {
    return [];
  }
}

export function writeDrawerPinnedItems(items: DrawerPinnedItem[], storage: Storage | null = browserStorage()) {
  storage?.setItem(drawerPinnedItemsKey, JSON.stringify(items));
}

export function toggleDrawerPinnedItem(items: DrawerPinnedItem[], item: DrawerPinnedItem): DrawerPinnedItem[] {
  return isDrawerPinned(items, item.id) ? items.filter((pinnedItem) => pinnedItem.id !== item.id) : [...items, item];
}

export function isDrawerPinned(items: DrawerPinnedItem[], id: string) {
  return items.some((item) => item.id === id);
}

function isDrawerPinnedItem(value: unknown): value is DrawerPinnedItem {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<DrawerPinnedItem>;
  return typeof candidate.id === "string" && typeof candidate.label === "string" && (candidate.type === "project" || candidate.type === "thread");
}

function browserStorage() {
  return typeof window === "undefined" ? null : window.localStorage;
}
