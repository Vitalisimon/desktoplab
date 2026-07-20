import type { MouseEvent } from "react";

export function startWindowDrag(event: MouseEvent<HTMLElement>) {
  void invokeWindowShellCommand("start_window_drag", event);
}

export function toggleWindowMaximized(event: MouseEvent<HTMLElement>) {
  void invokeWindowShellCommand("toggle_window_maximized", event);
}

async function invokeWindowShellCommand(command: "start_window_drag" | "toggle_window_maximized", event: MouseEvent<HTMLElement>) {
  if (event.button !== 0 || isInteractiveTarget(event.target)) return;
  if (!("__TAURI_INTERNALS__" in window)) return;
  event.preventDefault();
  await import("@tauri-apps/api/core")
    .then(({ invoke }) => invoke(command))
    .catch(() => undefined);
}

function isInteractiveTarget(target: EventTarget | null) {
  return target instanceof Element && Boolean(target.closest("button,a,input,select,textarea,[role='button']"));
}
