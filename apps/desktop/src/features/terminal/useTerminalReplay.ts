import { useEffect, useState } from "react";
import { useApiClient } from "../../api/ApiProvider";
import { terminalResponseFromFrames, type BackendEventFrame } from "../../api/events";

export function useTerminalReplay({
  open,
  enabled,
  refreshKey = 0,
}: {
  open: boolean;
  enabled: boolean;
  refreshKey?: number;
}) {
  const api = useApiClient();
  const [frames, setFrames] = useState<BackendEventFrame[]>([]);

  useEffect(() => {
    if (!open || !enabled) return;

    let active = true;
    let timer: number | undefined;
    const replay = async () => {
      try {
        const nextFrames = await api.replayEvents();
        if (active) setFrames(nextFrames);
      } catch {
        if (active) setFrames([]);
      } finally {
        if (active) timer = window.setTimeout(replay, 500);
      }
    };
    void replay();

    return () => {
      active = false;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [api, enabled, open, refreshKey]);

  return {
    frames,
    response: terminalResponseFromFrames(frames),
  };
}
