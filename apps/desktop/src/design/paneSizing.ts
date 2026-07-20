export const drawerWidth = {
  leftCollapsed: 64,
  leftDefault: 276,
  leftMin: 220,
  leftMax: 360,
  rightDefault: 420,
  rightMin: 260,
  rightMax: 720,
  terminalDefault: 132,
  terminalMin: 112,
  terminalMax: 420,
  handle: 4,
} as const;

export const paneStorageKeys = {
  leftWidth: "desktoplab.pane.leftWidth",
  rightWidth: "desktoplab.pane.rightWidth",
  terminalHeight: "desktoplab.pane.terminalHeight",
} as const;

export function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

export function readStoredPaneSize(key: string, fallback: number, min: number, max: number) {
  const stored = Number(window.localStorage.getItem(key));
  return Number.isFinite(stored) && stored > 0 ? clamp(stored, min, max) : fallback;
}

export function writeStoredPaneSize(key: string, value: number) {
  window.localStorage.setItem(key, String(value));
}
